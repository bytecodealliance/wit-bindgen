#[cfg(feature = "clap")]
use anyhow::anyhow;
use anyhow::Result;
use heck::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::mem;
use wasmtime_environ::component::{
    CanonicalOptions, Component, CoreDef, CoreExport, Export, ExportItem, GlobalInitializer,
    InstantiateModule, LowerImport, RuntimeInstanceIndex, StaticModuleIndex, StringEncoding,
};
use wasmtime_environ::{EntityIndex, ModuleTranslation, PrimaryMap};
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator, WorldGenerator,
};

#[derive(Default)]
struct Js {
    /// The source code for the "main" file that's going to be created for the
    /// component we're generating bindings for. This is incrementally added to
    /// over time and primarily contains the main `instantiate` function as well
    /// as a type-description of the input/output interfaces.
    src: Source,

    /// JS output imports map from imported specifier, to a list of bindings
    imports: HashMap<String, Vec<(String, String)>>,

    /// Type script definitions which will become the import object
    import_object: wit_bindgen_core::Source,
    /// Type script definitions which will become the export object
    export_object: wit_bindgen_core::Source,

    /// Core module count
    core_module_cnt: usize,

    /// Various options for code generation.
    opts: Opts,

    /// List of all intrinsics emitted to `src` so far.
    all_intrinsics: BTreeSet<Intrinsic>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Disables generation of `*.d.ts` files and instead only generates `*.js`
    /// source files.
    #[cfg_attr(feature = "clap", arg(long))]
    pub no_typescript: bool,
    /// Provide a custom JS instantiation API for the component instead
    /// of the direct importable native ESM output.
    #[cfg_attr(
        feature = "clap",
        arg(
            long,
            short = 'I',
            conflicts_with = "compatibility",
            conflicts_with = "no-compatibility",
            conflicts_with = "compat"
        )
    )]
    pub instantiation: bool,
    /// Comma-separated list of "from-specifier=./to-specifier.js" mappings of
    /// component import specifiers to JS import specifiers.
    #[cfg_attr(feature = "clap", arg(long), clap(value_parser = maps_str_to_map))]
    pub map: Option<HashMap<String, String>>,
    /// Enables all compat flags: --tla-compat.
    #[cfg_attr(feature = "clap", arg(long, short = 'c'))]
    pub compat: bool,
    /// Disables compatibility in Node.js without a fetch global.
    #[cfg_attr(feature = "clap", arg(long, group = "no-compatibility"))]
    pub no_nodejs_compat: bool,
    /// Set the cutoff byte size for base64 inlining core Wasm in instantiation mode
    /// (set to 0 to disable all base64 inlining)
    #[cfg_attr(feature = "clap", arg(long, short = 'b', default_value_t = 5000))]
    pub base64_cutoff: usize,
    /// Enables compatibility for JS environments without top-level await support
    /// via an async $init promise export to wait for instead.
    #[cfg_attr(feature = "clap", arg(long, group = "compatibility"))]
    pub tla_compat: bool,
    /// Disable verification of component Wasm data structures when
    /// lifting as a production optimization
    #[cfg_attr(feature = "clap", arg(long))]
    pub valid_lifting_optimization: bool,
}

impl Opts {
    pub fn build(self) -> Result<Box<dyn ComponentGenerator>> {
        let mut gen = Js::default();
        gen.opts = self;
        if gen.opts.compat {
            gen.opts.tla_compat = true;
        }
        Ok(Box::new(gen))
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum Intrinsic {
    Base64Compile,
    ClampGuest,
    ComponentError,
    DataView,
    F32ToI32,
    F64ToI64,
    FetchCompile,
    GetErrorPayload,
    HasOwnProperty,
    I32ToF32,
    I64ToF64,
    InstantiateCore,
    IsLE,
    ThrowInvalidBool,
    ThrowUninitialized,
    /// Implementation of https://tc39.es/ecma262/#sec-tobigint64.
    ToBigInt64,
    /// Implementation of https://tc39.es/ecma262/#sec-tobiguint64.
    ToBigUint64,
    /// Implementation of https://tc39.es/ecma262/#sec-toint16.
    ToInt16,
    /// Implementation of https://tc39.es/ecma262/#sec-toint32.
    ToInt32,
    /// Implementation of https://tc39.es/ecma262/#sec-toint8.
    ToInt8,
    /// Implementation of https://tc39.es/ecma262/#sec-tostring.
    ToString,
    /// Implementation of https://tc39.es/ecma262/#sec-touint16.
    ToUint16,
    /// Implementation of https://tc39.es/ecma262/#sec-touint32.
    ToUint32,
    /// Implementation of https://tc39.es/ecma262/#sec-touint8.
    ToUint8,
    Utf16Decoder,
    Utf16Encode,
    Utf8Decoder,
    Utf8Encode,
    Utf8EncodedLen,
    ValidateGuestChar,
    ValidateHostChar,
}

impl Intrinsic {
    fn name(&self) -> &'static str {
        match self {
            Intrinsic::Base64Compile => "base64Compile",
            Intrinsic::ClampGuest => "clampGuest",
            Intrinsic::ComponentError => "ComponentError",
            Intrinsic::DataView => "dataView",
            Intrinsic::F32ToI32 => "f32ToI32",
            Intrinsic::F64ToI64 => "f64ToI64",
            Intrinsic::GetErrorPayload => "getErrorPayload",
            Intrinsic::HasOwnProperty => "hasOwnProperty",
            Intrinsic::I32ToF32 => "i32ToF32",
            Intrinsic::I64ToF64 => "i64ToF64",
            Intrinsic::InstantiateCore => "instantiateCore",
            Intrinsic::IsLE => "isLE",
            Intrinsic::FetchCompile => "fetchCompile",
            Intrinsic::ThrowInvalidBool => "throwInvalidBool",
            Intrinsic::ThrowUninitialized => "throwUninitialized",
            Intrinsic::ToBigInt64 => "toInt64",
            Intrinsic::ToBigUint64 => "toUint64",
            Intrinsic::ToInt16 => "toInt16",
            Intrinsic::ToInt32 => "toInt32",
            Intrinsic::ToInt8 => "toInt8",
            Intrinsic::ToString => "toString",
            Intrinsic::ToUint16 => "toUint16",
            Intrinsic::ToUint32 => "toUint32",
            Intrinsic::ToUint8 => "toUint8",
            Intrinsic::Utf16Decoder => "utf16Decoder",
            Intrinsic::Utf16Encode => "utf16Encode",
            Intrinsic::Utf8Decoder => "utf8Decoder",
            Intrinsic::Utf8Encode => "utf8Encode",
            Intrinsic::Utf8EncodedLen => "utf8EncodedLen",
            Intrinsic::ValidateGuestChar => "validateGuestChar",
            Intrinsic::ValidateHostChar => "validateHostChar",
        }
    }
}

/// Used to generate a `*.d.ts` file for each imported and exported interface for
/// a component.
///
/// This generated source does not contain any actual JS runtime code, it's just
/// typescript definitions.
struct JsInterface<'a> {
    src: Source,
    gen: &'a mut Js,
    iface: &'a Interface,
    needs_ty_option: bool,
    needs_ty_result: bool,
}

impl ComponentGenerator for Js {
    fn instantiate(
        &mut self,
        component: &Component,
        modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
        world: &World,
    ) {
        self.core_module_cnt = modules.len();

        // Generate the TypeScript definition of the `instantiate` function
        // which is the main workhorse of the generated bindings.
        if self.opts.instantiation {
            let camel = world.name.to_upper_camel_case();
            uwriteln!(
                self.src.ts,
                "
                /**
                    * Instantiates this component with the provided imports and
                    * returns a map of all the exports of the component.
                    *
                    * This function is intended to be similar to the
                    * `WebAssembly.instantiate` function. The second `imports`
                    * argument is the \"import object\" for wasm, except here it
                    * uses component-model-layer types instead of core wasm
                    * integers/numbers/etc.
                    *
                    * The first argument to this function, `compileCore`, is
                    * used to compile core wasm modules within the component.
                    * Components are composed of core wasm modules and this callback
                    * will be invoked per core wasm module. The caller of this
                    * function is responsible for reading the core wasm module
                    * identified by `path` and returning its compiled
                    * WebAssembly.Module object. This would use `compileStreaming`
                    * on the web, for example.
                    */
                    export function instantiate(
                        compileCore: (path: string, imports: Record<string, any>) => Promise<WebAssembly.Module>,
                        imports: typeof ImportObject,
                        instantiateCore?: (module: WebAssembly.Module, imports: Record<string, any>) => Promise<WebAssembly.Instance>
                    ): Promise<typeof {camel}>;
                ",
            );
        }

        // bindings is the actual `instantiate` method itself, created by this
        // structure.
        let mut instantiator = Instantiator {
            src: Source::default(),
            gen: self,
            modules,
            instances: Default::default(),
            world,
            component,
        };
        instantiator.instantiate();
        instantiator.gen.src.js(&instantiator.src.js);
        instantiator.gen.src.js_init(&instantiator.src.js_init);
        assert!(instantiator.src.ts.is_empty());
    }

    fn finish_component(&mut self, name: &str, files: &mut Files) {
        let mut output = wit_bindgen_core::Source::default();
        let mut compilation_promises = wit_bindgen_core::Source::default();

        // Setup the compilation data and compilation promises
        let mut removed = BTreeSet::new();
        for i in 0..self.core_module_cnt {
            let local_name = format!("module{}", i);
            let mut name_idx = self.core_file_name(name, i as u32);
            if self.opts.instantiation {
                uwriteln!(
                    compilation_promises,
                    "const {local_name} = compileCore('{name_idx}');"
                );
            } else {
                if files.get_size(&name_idx).unwrap() < self.opts.base64_cutoff {
                    assert!(removed.insert(i));
                    let data = files.remove(&name_idx).unwrap();
                    uwriteln!(
                        compilation_promises,
                        "const {local_name} = {}('{}');",
                        self.intrinsic(Intrinsic::Base64Compile),
                        base64::encode(&data)
                    );
                } else {
                    // Maintain numerical file orderings when a previous file was
                    // inlined
                    if let Some(&replacement) = removed.iter().next() {
                        assert!(removed.remove(&replacement) && removed.insert(i));
                        let data = files.remove(&name_idx).unwrap();
                        name_idx = self.core_file_name(name, replacement as u32);
                        files.push(&name_idx, &data);
                    }
                    uwriteln!(
                        compilation_promises,
                        "const {local_name} = {}(new URL('./{name_idx}', import.meta.url));",
                        self.intrinsic(Intrinsic::FetchCompile)
                    );
                }
            }
        }

        if self.opts.instantiation {
            uwrite!(
                output,
                "\
                    {}
                    export async function instantiate(compileCore, imports, instantiateCore = WebAssembly.instantiate) {{
                        {}
                        {}\
                        {};
                    }}
                ",
                &self.src.js_intrinsics as &str,
                &compilation_promises as &str,
                &self.src.js_init as &str,
                &self.src.js as &str,
            );
        } else {
            // Import statements render first in JS instance mode
            for (specifier, bindings) in &self.imports {
                uwrite!(output, "import {{");
                let mut first = true;
                for (external, local) in bindings {
                    if first {
                        output.push_str(" ");
                    } else {
                        output.push_str(", ");
                    }
                    uwrite!(output, "{} as {}", external, local);
                    first = false;
                }
                if !first {
                    output.push_str(" ");
                }
                uwrite!(output, "}} from '{}';\n", specifier);
            }

            let (maybe_init_export, maybe_init) = if self.opts.tla_compat {
                uwriteln!(
                    self.src.ts,
                    "
                    export const $init: Promise<void>;"
                );
                uwriteln!(self.src.js_init, "_initialized = true;");
                (
                    "\
                        let _initialized = false;
                        export ",
                    "",
                )
            } else {
                (
                    "",
                    "
                        await $init;
                    ",
                )
            };

            uwrite!(
                output,
                "\
                    {}
                    {}
                    {maybe_init_export}const $init = (async() => {{
                        {}\
                        {}\
                    }})();
                    {maybe_init}\
                ",
                &self.src.js_intrinsics as &str,
                &self.src.js as &str,
                &compilation_promises as &str,
                &self.src.js_init as &str,
            );
        }

        let mut bytes = output.as_bytes();
        // strip leading newline
        if bytes[0] == b'\n' {
            bytes = &bytes[1..];
        }
        files.push(&format!("{name}.js"), bytes);
        if !self.opts.no_typescript {
            files.push(&format!("{name}.d.ts"), self.src.ts.as_bytes());
        }
    }
}

impl WorldGenerator for Js {
    fn import(&mut self, name: &str, iface: &Interface, files: &mut Files) {
        self.generate_interface(
            name,
            iface,
            "imports",
            "Imports",
            files,
            AbiVariant::GuestImport,
        );
        let camel = name.to_upper_camel_case();
        uwriteln!(
            self.import_object,
            "export const {name}: typeof {camel}Imports;"
        );
    }

    fn export(&mut self, name: &str, iface: &Interface, files: &mut Files) {
        self.generate_interface(
            name,
            iface,
            "exports",
            "Exports",
            files,
            AbiVariant::GuestExport,
        );
        let camel = name.to_upper_camel_case();
        uwriteln!(self.src.ts, "export const {name}: typeof {camel}Exports;");
    }

    fn export_default(&mut self, _name: &str, iface: &Interface, _files: &mut Files) {
        let instantiation = self.opts.instantiation;
        let mut gen = self.js_interface(iface);
        for func in iface.functions.iter() {
            gen.ts_func(func, AbiVariant::GuestExport);
        }
        if instantiation {
            gen.gen.export_object.push_str(&mem::take(&mut gen.src.ts));
        }

        // After the default interface has its function definitions
        // inlined the rest of the types are generated here as well.
        gen.types();
        gen.post_types();
        gen.gen.src.ts(&mem::take(&mut gen.src.ts));
    }

    fn finish(&mut self, world: &World, _files: &mut Files) {
        let camel = world.name.to_upper_camel_case();

        // Generate a type definition for the import object to type-check
        // all imports to the component.
        //
        // With the current representation of a "world" this is an import object
        // per-imported-interface where the type of that field is defined by the
        // interface itself.
        if self.opts.instantiation {
            uwriteln!(self.src.ts, "export namespace ImportObject {{");
            self.src.ts(&self.import_object);
            uwriteln!(self.src.ts, "}}");
        }

        // Generate a type definition for the export object from instantiating
        // the component.
        if self.opts.instantiation {
            uwriteln!(self.src.ts, "export namespace {camel} {{",);
            self.src.ts(&self.export_object);
            uwriteln!(self.src.ts, "}}");
        }
    }
}

impl Js {
    fn generate_interface(
        &mut self,
        name: &str,
        iface: &Interface,
        dir: &str,
        extra: &str,
        files: &mut Files,
        abi: AbiVariant,
    ) {
        let camel = name.to_upper_camel_case();
        let mut gen = self.js_interface(iface);
        gen.types();
        gen.post_types();

        uwriteln!(gen.src.ts, "export namespace {camel} {{");
        for func in iface.functions.iter() {
            gen.ts_func(func, abi);
        }
        uwriteln!(gen.src.ts, "}}");

        assert!(gen.src.js.is_empty());
        if !gen.gen.opts.no_typescript {
            files.push(&format!("{dir}/{name}.d.ts"), gen.src.ts.as_bytes());
        }

        uwriteln!(
            self.src.ts,
            "{} {{ {camel} as {camel}{extra} }} from './{dir}/{name}';",
            // In instance mode, we have no way to assert the imported types
            // in the ambient declaration file. Instead we just export the
            // import namespace types for users to use.
            if self.opts.instantiation {
                "import"
            } else {
                "export"
            }
        );
    }

    fn map_import(&self, impt: &str) -> String {
        if let Some(map) = self.opts.map.as_ref() {
            if let Some(mapping) = map.get(impt) {
                return mapping.into();
            }
        }
        impt.into()
    }

    fn js_interface<'a>(&'a mut self, iface: &'a Interface) -> JsInterface<'a> {
        JsInterface {
            src: Source::default(),
            gen: self,
            iface,
            needs_ty_option: false,
            needs_ty_result: false,
        }
    }

    /// Emits the intrinsic `i` to this file and then returns the name of the
    /// intrinsic.
    fn intrinsic(&mut self, i: Intrinsic) -> String {
        let name = i.name().to_string();
        if !self.all_intrinsics.insert(i) {
            return name;
        }

        if (i == Intrinsic::I32ToF32 && !self.all_intrinsics.contains(&Intrinsic::F32ToI32))
            || (i == Intrinsic::F32ToI32 && !self.all_intrinsics.contains(&Intrinsic::I32ToF32))
        {
            self.src.js_intrinsics(
                "
                const i32ToF32I = new Int32Array(1);
                const i32ToF32F = new Float32Array(i32ToF32I.buffer);
            ",
            );
        }
        if (i == Intrinsic::I64ToF64 && !self.all_intrinsics.contains(&Intrinsic::F64ToI64))
            || (i == Intrinsic::F64ToI64 && !self.all_intrinsics.contains(&Intrinsic::I64ToF64))
        {
            self.src.js_intrinsics(
                "
                const i64ToF64I = new BigInt64Array(1);
                const i64ToF64F = new Float64Array(i64ToF64I.buffer);
            ",
            );
        }

        match i {
            Intrinsic::ClampGuest => self.src.js_intrinsics("
                function clampGuest(i, min, max) {
                    if (i < min || i > max) \
                        throw new TypeError(`must be between ${min} and ${max}`);
                    return i;
                }
            "),

            Intrinsic::HasOwnProperty => self.src.js_intrinsics("
                const hasOwnProperty = Object.prototype.hasOwnProperty;
            "),

            Intrinsic::GetErrorPayload => {
                let hop = self.intrinsic(Intrinsic::HasOwnProperty);
                uwrite!(self.src.js_intrinsics, "
                    function getErrorPayload(e) {{
                        if ({hop}.call(e, 'payload')) return e.payload;
                        if ({hop}.call(e, 'message')) return String(e.message);
                        return String(e);
                    }}
                ")
            },

            Intrinsic::ComponentError => self.src.js_intrinsics("
                class ComponentError extends Error {
                    constructor (value) {
                        const enumerable = typeof value !== 'string';
                        super(enumerable ? `${String(value)} (see error.payload)` : value);
                        Object.defineProperty(this, 'payload', { value, enumerable });
                    }
                }
            "),

            Intrinsic::DataView => self.src.js_intrinsics("
                let dv = new DataView(new ArrayBuffer());
                const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);
            "),

            Intrinsic::FetchCompile => if !self.opts.no_nodejs_compat {
                self.src.js_intrinsics("
                    const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
                    let _fs;
                    async function fetchCompile (url) {
                        if (isNode) {
                            _fs = _fs || await import('fs/promises');
                            return WebAssembly.compile(await _fs.readFile(url));
                        }
                        return fetch(url).then(WebAssembly.compileStreaming);
                    }
                ")
            } else {
                self.src.js_intrinsics("
                    const fetchCompile = url => fetch(url).then(WebAssembly.compileStreaming);
                ")
            },

            Intrinsic::Base64Compile => if !self.opts.no_nodejs_compat {
                self.src.js_intrinsics("
                    const base64Compile = str => WebAssembly.compile(typeof Buffer !== 'undefined' ? Buffer.from(str, 'base64') : Uint8Array.from(atob(str), b => b.charCodeAt(0)));
                ")
            } else {
                self.src.js_intrinsics("
                    const base64Compile = str => WebAssembly.compile(Uint8Array.from(atob(str), b => b.charCodeAt(0)));
                ")
            },

            Intrinsic::InstantiateCore => if !self.opts.instantiation {
                self.src.js_intrinsics("
                    const instantiateCore = WebAssembly.instantiate;
                ")
            },

            Intrinsic::IsLE => self.src.js_intrinsics("
                const isLE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
            "),

            Intrinsic::ValidateGuestChar => self.src.js_intrinsics("
                function validateGuestChar(i) {
                    if ((i > 0x10ffff) || (i >= 0xd800 && i <= 0xdfff)) \
                        throw new TypeError(`not a valid char`);
                    return String.fromCodePoint(i);
                }
            "),

            // TODO: this is incorrect. It at least allows strings of length > 0
            // but it probably doesn't do the right thing for unicode or invalid
            // utf16 strings either.
            Intrinsic::ValidateHostChar => self.src.js_intrinsics("
                function validateHostChar(s) {
                    if (typeof s !== 'string') \
                        throw new TypeError(`must be a string`);
                    return s.codePointAt(0);
                }
            "),


            Intrinsic::ToInt32 => self.src.js_intrinsics("
                function toInt32(val) {
                    return val >> 0;
                }
            "),
            Intrinsic::ToUint32 => self.src.js_intrinsics("
                function toUint32(val) {
                    return val >>> 0;
                }
            "),

            Intrinsic::ToInt16 => self.src.js_intrinsics("
                function toInt16(val) {
                    val >>>= 0;
                    val %= 2 ** 16;
                    if (val >= 2 ** 15) {
                        val -= 2 ** 16;
                    }
                    return val;
                }
            "),
            Intrinsic::ToUint16 => self.src.js_intrinsics("
                function toUint16(val) {
                    val >>>= 0;
                    val %= 2 ** 16;
                    return val;
                }
            "),
            Intrinsic::ToInt8 => self.src.js_intrinsics("
                function toInt8(val) {
                    val >>>= 0;
                    val %= 2 ** 8;
                    if (val >= 2 ** 7) {
                        val -= 2 ** 8;
                    }
                    return val;
                }
            "),
            Intrinsic::ToUint8 => self.src.js_intrinsics("
                function toUint8(val) {
                    val >>>= 0;
                    val %= 2 ** 8;
                    return val;
                }
            "),

            Intrinsic::ToBigInt64 => self.src.js_intrinsics("
                const toInt64 = val => BigInt.asIntN(64, val);
            "),
            Intrinsic::ToBigUint64 => self.src.js_intrinsics("
                const toUint64 = val => BigInt.asUintN(64, val);
            "),

            // Calling `String` almost directly calls `ToString`, except that it also allows symbols,
            // which is why we have the symbol-rejecting branch above.
            //
            // Definition of `String`: https://tc39.es/ecma262/#sec-string-constructor-string-value
            Intrinsic::ToString => self.src.js_intrinsics("
                function toString(val) {
                    if (typeof val === 'symbol') throw new TypeError('symbols cannot be converted to strings');
                    return String(val);
                }
            "),

            Intrinsic::I32ToF32 => self.src.js_intrinsics("
                const i32ToF32 = i => (i32ToF32I[0] = i, i32ToF32F[0]);
            "),
            Intrinsic::F32ToI32 => self.src.js_intrinsics("
                const f32ToI32 = f => (i32ToF32F[0] = f, i32ToF32I[0]);
            "),
            Intrinsic::I64ToF64 => self.src.js_intrinsics("
                const i64ToF64 = i => (i64ToF64I[0] = i, i64ToF64F[0]);
            "),
            Intrinsic::F64ToI64 => self.src.js_intrinsics("
                const f64ToI64 = f => (i64ToF64F[0] = f, i64ToF64I[0]);
            "),

            Intrinsic::Utf8Decoder => self.src.js_intrinsics("
                const utf8Decoder = new TextDecoder();
            "),

            Intrinsic::Utf16Decoder => self.src.js_intrinsics("
                const utf16Decoder = new TextDecoder('utf-16');
            "),

            Intrinsic::Utf8EncodedLen => {},

            Intrinsic::Utf8Encode => self.src.js_intrinsics("
                const utf8Encoder = new TextEncoder();

                let utf8EncodedLen = 0;
                function utf8Encode(s, realloc, memory) {
                    if (typeof s !== 'string') \
                        throw new TypeError('expected a string');
                    if (s.length === 0) {
                        utf8EncodedLen = 0;
                        return 1;
                    }
                    let allocLen = 0;
                    let ptr = 0;
                    let writtenTotal = 0;
                    while (s.length > 0) {
                        ptr = realloc(ptr, allocLen, 1, allocLen + s.length);
                        allocLen += s.length;
                        const { read, written } = utf8Encoder.encodeInto(
                            s,
                            new Uint8Array(memory.buffer, ptr + writtenTotal, allocLen - writtenTotal),
                        );
                        writtenTotal += written;
                        s = s.slice(read);
                    }
                    if (allocLen > writtenTotal)
                        ptr = realloc(ptr, allocLen, 1, writtenTotal);
                    utf8EncodedLen = writtenTotal;
                    return ptr;
                }
            "),

            Intrinsic::Utf16Encode => {
                let is_le = self.intrinsic(Intrinsic::IsLE);
                uwrite!(self.src.js_intrinsics, "
                    function utf16Encode (str, realloc, memory) {{
                        const len = str.length, ptr = realloc(0, 0, 2, len * 2), out = new Uint16Array(memory.buffer, ptr, len);
                        let i = 0;
                        if ({is_le}) {{
                            while (i < len) out[i] = str.charCodeAt(i++);
                        }} else {{
                            while (i < len) {{
                                const ch = str.charCodeAt(i);
                                out[i++] = (ch & 0xff) << 8 | ch >>> 8;
                            }}
                        }}
                        return ptr;
                    }}
                ");
            },

            Intrinsic::ThrowInvalidBool => self.src.js_intrinsics("
                function throwInvalidBool() {
                    throw new TypeError('invalid variant discriminant for bool');
                }
            "),

            Intrinsic::ThrowUninitialized => self.src.js_intrinsics("
                function throwUninitialized() {
                    throw new TypeError('Wasm uninitialized use `await $init` first');
                }
            "),
        }

        name
    }

    fn array_ty(&self, iface: &Interface, ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Bool => None,
            Type::U8 => Some("Uint8Array"),
            Type::S8 => Some("Int8Array"),
            Type::U16 => Some("Uint16Array"),
            Type::S16 => Some("Int16Array"),
            Type::U32 => Some("Uint32Array"),
            Type::S32 => Some("Int32Array"),
            Type::U64 => Some("BigUint64Array"),
            Type::S64 => Some("BigInt64Array"),
            Type::Float32 => Some("Float32Array"),
            Type::Float64 => Some("Float64Array"),
            Type::Char => None,
            Type::String => None,
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Type(t) => self.array_ty(iface, t),
                _ => None,
            },
        }
    }

    /// Returns whether `null` is a valid value of type `ty`
    fn maybe_null(&self, iface: &Interface, ty: &Type) -> bool {
        self.as_nullable(iface, ty).is_some()
    }

    /// Tests whether `ty` can be represented with `null`, and if it can then
    /// the "other type" is returned. If `Some` is returned that means that `ty`
    /// is `null | <return>`. If `None` is returned that means that `null` can't
    /// be used to represent `ty`.
    fn as_nullable<'a>(&self, iface: &'a Interface, ty: &'a Type) -> Option<&'a Type> {
        let id = match ty {
            Type::Id(id) => *id,
            _ => return None,
        };
        match &iface.types[id].kind {
            // If `ty` points to an `option<T>`, then `ty` can be represented
            // with `null` if `t` itself can't be represented with null. For
            // example `option<option<u32>>` can't be represented with `null`
            // since that's ambiguous if it's `none` or `some(none)`.
            //
            // Note, oddly enough, that `option<option<option<u32>>>` can be
            // represented as `null` since:
            //
            // * `null` => `none`
            // * `{ tag: "none" }` => `some(none)`
            // * `{ tag: "some", val: null }` => `some(some(none))`
            // * `{ tag: "some", val: 1 }` => `some(some(some(1)))`
            //
            // It's doubtful anyone would actually rely on that though due to
            // how confusing it is.
            TypeDefKind::Option(t) => {
                if !self.maybe_null(iface, t) {
                    Some(t)
                } else {
                    None
                }
            }
            TypeDefKind::Type(t) => self.as_nullable(iface, t),
            _ => None,
        }
    }
}

/// Helper structure used to generate the `instantiate` method of a component.
///
/// This is the main structure for parsing the output of Wasmtime.
struct Instantiator<'a> {
    src: Source,
    gen: &'a mut Js,
    modules: &'a PrimaryMap<StaticModuleIndex, ModuleTranslation<'a>>,
    instances: PrimaryMap<RuntimeInstanceIndex, StaticModuleIndex>,
    world: &'a World,
    component: &'a Component,
}

impl Instantiator<'_> {
    fn instantiate(&mut self) {
        // To avoid uncaught promise rejection errors, we attach an intermediate
        // Promise.all with a rejection handler, if there are multiple promises.
        if self.modules.len() > 1 {
            self.src.js_init.push_str("Promise.all([");
            for i in 0..self.modules.len() {
                if i > 0 {
                    self.src.js_init.push_str(", ");
                }
                self.src.js_init.push_str(&format!("module{}", i));
            }
            uwriteln!(self.src.js_init, "]).catch(() => {{}});");
        }

        for init in self.component.initializers.iter() {
            self.instantiation_global_initializer(init);
        }

        if self.gen.opts.instantiation {
            let js_init = mem::take(&mut self.src.js_init);
            self.src.js.push_str(&js_init);
            self.src.js("return ");
        }

        self.exports(
            &self.component.exports,
            0,
            self.world.default.as_ref().map(|i| (None, i)),
        );
    }

    fn instantiation_global_initializer(&mut self, init: &GlobalInitializer) {
        match init {
            GlobalInitializer::InstantiateModule(m) => match m {
                InstantiateModule::Static(idx, args) => self.instantiate_static_module(*idx, args),
                // This is only needed when instantiating an imported core wasm
                // module which while easy to implement here is not possible to
                // test at this time so it's left unimplemented.
                InstantiateModule::Import(..) => unimplemented!(),
            },
            GlobalInitializer::LowerImport(i) => {
                self.lower_import(i);
            }
            GlobalInitializer::ExtractMemory(m) => {
                let def = self.core_export(&m.export);
                let idx = m.index.as_u32();
                uwriteln!(self.src.js, "let memory{idx};");
                uwriteln!(self.src.js_init, "memory{idx} = {def};");
            }
            GlobalInitializer::ExtractRealloc(r) => {
                let def = self.core_def(&r.def);
                let idx = r.index.as_u32();
                uwriteln!(self.src.js, "let realloc{idx};");
                uwriteln!(self.src.js_init, "realloc{idx} = {def};",);
            }
            GlobalInitializer::ExtractPostReturn(p) => {
                let def = self.core_def(&p.def);
                let idx = p.index.as_u32();
                uwriteln!(self.src.js, "let postReturn{idx};");
                uwriteln!(self.src.js_init, "postReturn{idx} = {def};");
            }

            // This is only used for a "degenerate component" which internally
            // has a function that always traps. While this should be trivial to
            // implement (generate a JS function that always throws) there's no
            // way to test this at this time so leave this unimplemented.
            GlobalInitializer::AlwaysTrap(_) => unimplemented!(),

            // This is only used when the component exports core wasm modules,
            // but that's not possible to test right now so leave these as
            // unimplemented.
            GlobalInitializer::SaveStaticModule(_) => unimplemented!(),
            GlobalInitializer::SaveModuleImport(_) => unimplemented!(),

            // This is required when strings pass between components within a
            // component and may change encodings. This is left unimplemented
            // for now since it can't be tested and additionally JS doesn't
            // support multi-memory which transcoders rely on anyway.
            GlobalInitializer::Transcoder(_) => unimplemented!(),
        }
    }

    fn instantiate_static_module(&mut self, idx: StaticModuleIndex, args: &[CoreDef]) {
        let module = &self.modules[idx].module;

        // Build a JS "import object" which represents `args`. The `args` is a
        // flat representation which needs to be zip'd with the list of names to
        // correspond to the JS wasm embedding API. This is one of the major
        // differences between Wasmtime's and JS's embedding API.
        let mut import_obj = BTreeMap::new();
        assert_eq!(module.imports().len(), args.len());
        for ((module, name, _), arg) in module.imports().zip(args) {
            let def = self.core_def(arg);
            let dst = import_obj.entry(module).or_insert(BTreeMap::new());
            let prev = dst.insert(name, def);
            assert!(prev.is_none());
        }
        let mut imports = String::new();
        if !import_obj.is_empty() {
            imports.push_str(", {\n");
            for (module, names) in import_obj {
                if is_js_identifier(module) {
                    imports.push_str(module);
                } else {
                    uwrite!(imports, "'{module}'");
                }
                imports.push_str(": {\n");
                for (name, val) in names {
                    if is_js_identifier(name) {
                        imports.push_str(name);
                    } else {
                        uwrite!(imports, "'{name}'");
                    }
                    uwriteln!(imports, ": {val},");
                }
                imports.push_str("},\n");
            }
            imports.push_str("}");
        }

        let i = self.instances.push(idx);
        let iu32 = i.as_u32();
        let instantiate = self.gen.intrinsic(Intrinsic::InstantiateCore);
        uwriteln!(self.src.js, "let exports{iu32};");
        uwriteln!(
            self.src.js_init,
            "
                ({{ exports: exports{iu32} }} = await {instantiate}(await module{}{imports}));\
            ",
            idx.as_u32()
        );
    }

    fn lower_import(&mut self, import: &LowerImport) {
        // Determine the `Interface` that this import corresponds to. At this
        // time `wit-component` only supports root-level imports of instances
        // where instances export functions.
        let (import_index, path) = &self.component.imports[import.import];
        let (import_name, _import_ty) = &self.component.import_types[*import_index];
        assert_eq!(path.len(), 1);
        let iface = &self.world.imports[import_name.as_str()];
        let func = iface.functions.iter().find(|f| f.name == path[0]).unwrap();

        let index = import.index.as_u32();
        let callee = format!("lowering{index}Callee");

        let import_specifier = self.gen.map_import(import_name);

        let id = func.name.to_lower_camel_case();

        // instance imports are otherwise hoisted
        if self.gen.opts.instantiation {
            uwriteln!(
                self.src.js,
                "const {callee} = imports{}.{};",
                if is_js_identifier(&import_specifier) {
                    format!(".{}", import_specifier)
                } else {
                    format!("['{}']", import_specifier)
                },
                id
            );
        } else {
            let imports_vec = self
                .gen
                .imports
                .entry(import_specifier)
                .or_insert(Vec::new());
            imports_vec.push((id, callee.clone()));
        }

        uwrite!(self.src.js_init, "\nfunction lowering{index}");
        let nparams = iface
            .wasm_signature(AbiVariant::GuestImport, func)
            .params
            .len();
        let prev = mem::take(&mut self.src);
        self.bindgen(
            nparams,
            callee,
            &import.options,
            iface,
            func,
            AbiVariant::GuestImport,
        );
        let latest = mem::replace(&mut self.src, prev);
        assert!(latest.ts.is_empty());
        assert!(latest.js_init.is_empty());
        self.src.js_intrinsics(&latest.js_intrinsics);
        self.src.js_init(&latest.js);
        uwriteln!(self.src.js_init, "");
    }

    fn bindgen(
        &mut self,
        nparams: usize,
        callee: String,
        opts: &CanonicalOptions,
        iface: &Interface,
        func: &Function,
        abi: AbiVariant,
    ) {
        let memory = match opts.memory {
            Some(idx) => Some(format!("memory{}", idx.as_u32())),
            None => None,
        };
        let realloc = match opts.realloc {
            Some(idx) => Some(format!("realloc{}", idx.as_u32())),
            None => None,
        };
        let post_return = match opts.post_return {
            Some(idx) => Some(format!("postReturn{}", idx.as_u32())),
            None => None,
        };

        self.src.js("(");
        let mut params = Vec::new();
        for i in 0..nparams {
            if i > 0 {
                self.src.js(", ");
            }
            let param = format!("arg{i}");
            self.src.js(&param);
            params.push(param);
        }
        uwriteln!(self.src.js, ") {{");

        if self.gen.opts.tla_compat && matches!(abi, AbiVariant::GuestExport) {
            let throw_uninitialized = self.gen.intrinsic(Intrinsic::ThrowUninitialized);
            uwrite!(
                self.src.js,
                "\
                if (!_initialized) {throw_uninitialized}();
            "
            );
        }

        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        let mut f = FunctionBindgen {
            sizes,
            gen: self.gen,
            err: if func.results.throws(iface).is_some() {
                match abi {
                    AbiVariant::GuestExport => ErrHandling::ThrowResultErr,
                    AbiVariant::GuestImport => ErrHandling::ResultCatchHandler,
                }
            } else {
                ErrHandling::None
            },
            block_storage: Vec::new(),
            blocks: Vec::new(),
            callee,
            memory,
            realloc,
            tmp: 0,
            params,
            post_return,
            encoding: opts.string_encoding,
            src: Source::default(),
        };
        iface.call(
            abi,
            match abi {
                AbiVariant::GuestImport => LiftLower::LiftArgsLowerResults,
                AbiVariant::GuestExport => LiftLower::LowerArgsLiftResults,
            },
            func,
            &mut f,
        );
        let FunctionBindgen { src, .. } = f;

        self.src.js(&src.js);
        assert!(src.ts.is_empty());
        self.src.js("}");
    }

    fn core_def(&self, def: &CoreDef) -> String {
        match def {
            CoreDef::Export(e) => self.core_export(e),
            CoreDef::Lowered(i) => format!("lowering{}", i.as_u32()),
            CoreDef::AlwaysTrap(_) => unimplemented!(),
            CoreDef::InstanceFlags(_) => unimplemented!(),
            CoreDef::Transcoder(_) => unimplemented!(),
        }
    }

    fn core_export<T>(&self, export: &CoreExport<T>) -> String
    where
        T: Into<EntityIndex> + Copy,
    {
        let name = match &export.item {
            ExportItem::Index(idx) => {
                let module = &self.modules[self.instances[export.instance]].module;
                let idx = (*idx).into();
                module
                    .exports
                    .iter()
                    .filter_map(|(name, i)| if *i == idx { Some(name) } else { None })
                    .next()
                    .unwrap()
            }
            ExportItem::Name(s) => s,
        };
        let i = export.instance.as_u32() as usize;
        if is_js_identifier(name) {
            format!("exports{i}.{name}")
        } else {
            format!("exports{i}['{name}']")
        }
    }

    fn exports(
        &mut self,
        exports: &IndexMap<String, Export>,
        depth: usize,
        iface: Option<(Option<&str>, &Interface)>,
    ) {
        if exports.is_empty() {
            if self.gen.opts.instantiation {
                self.src.js("{}");
            }
            return;
        }

        if self.gen.opts.instantiation {
            uwriteln!(self.src.js, "{{");
        }

        for (name, export) in exports {
            let js_name = if self.gen.opts.instantiation {
                name.clone()
            } else {
                // When generating direct ES-module exports namespace functions
                // by their exported interface name, if applicable.
                match iface {
                    Some((Some(iface_name), _)) => format!("{iface_name}-{name}"),
                    _ => name.clone(),
                }
            };
            let camel = js_name.to_lower_camel_case();
            match export {
                Export::LiftedFunction {
                    ty: _,
                    func,
                    options,
                } => {
                    assert!(depth < 2);
                    if self.gen.opts.instantiation {
                        uwrite!(self.src.js, "{camel}");
                    } else {
                        uwrite!(self.src.js, "\nexport function {camel}");
                    }
                    let callee = self.core_def(func);
                    let (_, iface) = iface.unwrap();
                    let func = iface.functions.iter().find(|f| f.name == *name).unwrap();
                    self.bindgen(
                        func.params.len(),
                        callee,
                        options,
                        iface,
                        func,
                        AbiVariant::GuestExport,
                    );
                    if self.gen.opts.instantiation {
                        self.src.js(",\n");
                    } else {
                        self.src.js("\n");
                    }
                }
                Export::Instance(exports) => {
                    if self.gen.opts.instantiation {
                        uwrite!(self.src.js, "{camel}: ");
                    }
                    let iface = &self.world.exports[name.as_str()];
                    self.exports(exports, depth + 1, Some((Some(name.as_str()), iface)));
                    if self.gen.opts.instantiation {
                        self.src.js(",\n");
                    }
                }

                // ignore type exports for now
                Export::Type(_) => {}

                // This can't be tested at this time so leave it unimplemented
                Export::Module(_) => unimplemented!(),
            }
        }
        if self.gen.opts.instantiation {
            self.src.js("}");
        }
    }
}

#[derive(Copy, Clone)]
enum Mode {
    Lift,
    Lower,
}

impl<'a> JsInterface<'a> {
    fn docs_raw(&mut self, docs: &str) {
        self.src.ts("/**\n");
        for line in docs.lines() {
            self.src.ts(&format!(" * {}\n", line));
        }
        self.src.ts(" */\n");
    }

    fn docs(&mut self, docs: &Docs) {
        match &docs.contents {
            Some(docs) => self.docs_raw(docs),
            None => return,
        }
    }

    fn array_ty(&self, ty: &Type) -> Option<&'static str> {
        self.gen.array_ty(self.iface, ty)
    }

    fn print_ty(&mut self, ty: &Type, mode: Mode) {
        match ty {
            Type::Bool => self.src.ts("boolean"),
            Type::U8
            | Type::S8
            | Type::U16
            | Type::S16
            | Type::U32
            | Type::S32
            | Type::Float32
            | Type::Float64 => self.src.ts("number"),
            Type::U64 | Type::S64 => self.src.ts("bigint"),
            Type::Char => self.src.ts("string"),
            Type::String => self.src.ts("string"),
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    return self.src.ts(&name.to_upper_camel_case());
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(t, mode),
                    TypeDefKind::Tuple(t) => self.print_tuple(t, mode),
                    TypeDefKind::Record(_) => panic!("anonymous record"),
                    TypeDefKind::Flags(_) => panic!("anonymous flags"),
                    TypeDefKind::Enum(_) => panic!("anonymous enum"),
                    TypeDefKind::Union(_) => panic!("anonymous union"),
                    TypeDefKind::Option(t) => {
                        if self.maybe_null(t) {
                            self.needs_ty_option = true;
                            self.src.ts("Option<");
                            self.print_ty(t, mode);
                            self.src.ts(">");
                        } else {
                            self.print_ty(t, mode);
                            self.src.ts(" | null");
                        }
                    }
                    TypeDefKind::Result(r) => {
                        self.needs_ty_result = true;
                        self.src.ts("Result<");
                        self.print_optional_ty(r.ok.as_ref(), mode);
                        self.src.ts(", ");
                        self.print_optional_ty(r.err.as_ref(), mode);
                        self.src.ts(">");
                    }
                    TypeDefKind::Variant(_) => panic!("anonymous variant"),
                    TypeDefKind::List(v) => self.print_list(v, mode),
                    TypeDefKind::Future(_) => todo!("anonymous future"),
                    TypeDefKind::Stream(_) => todo!("anonymous stream"),
                }
            }
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>, mode: Mode) {
        match ty {
            Some(ty) => self.print_ty(ty, mode),
            None => self.src.ts("void"),
        }
    }

    fn print_list(&mut self, ty: &Type, mode: Mode) {
        match self.array_ty(ty) {
            Some("Uint8Array") => match mode {
                Mode::Lift => self.src.ts("Uint8Array"),
                Mode::Lower => self.src.ts("Uint8Array | ArrayBuffer"),
            },
            Some(ty) => self.src.ts(ty),
            None => {
                self.print_ty(ty, mode);
                self.src.ts("[]");
            }
        }
    }

    fn print_tuple(&mut self, tuple: &Tuple, mode: Mode) {
        self.src.ts("[");
        for (i, ty) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.src.ts(", ");
            }
            self.print_ty(ty, mode);
        }
        self.src.ts("]");
    }

    fn ts_func(&mut self, func: &Function, abi: AbiVariant) {
        self.docs(&func.docs);

        self.src.ts("export function ");
        self.src.ts(&func.item_name().to_lower_camel_case());
        self.src.ts("(");

        let param_start = match &func.kind {
            FunctionKind::Freestanding => 0,
        };

        for (i, (name, ty)) in func.params[param_start..].iter().enumerate() {
            if i > 0 {
                self.src.ts(", ");
            }
            self.src.ts(to_js_ident(&name.to_lower_camel_case()));
            self.src.ts(": ");
            self.print_ty(
                ty,
                match abi {
                    AbiVariant::GuestExport => Mode::Lower,
                    AbiVariant::GuestImport => Mode::Lift,
                },
            );
        }
        self.src.ts("): ");
        let result_mode = match abi {
            AbiVariant::GuestExport => Mode::Lift,
            AbiVariant::GuestImport => Mode::Lower,
        };
        if let Some((ok_ty, _)) = func.results.throws(self.iface) {
            self.print_optional_ty(ok_ty, result_mode);
        } else {
            match func.results.len() {
                0 => self.src.ts("void"),
                1 => self.print_ty(func.results.iter_types().next().unwrap(), result_mode),
                _ => {
                    self.src.ts("[");
                    for (i, ty) in func.results.iter_types().enumerate() {
                        if i != 0 {
                            self.src.ts(", ");
                        }
                        self.print_ty(ty, result_mode);
                    }
                    self.src.ts("]");
                }
            }
        }
        self.src.ts(";\n");
    }

    fn maybe_null(&self, ty: &Type) -> bool {
        self.gen.maybe_null(self.iface, ty)
    }

    fn as_nullable<'b>(&self, ty: &'b Type) -> Option<&'b Type>
    where
        'a: 'b,
    {
        self.gen.as_nullable(self.iface, ty)
    }

    fn post_types(&mut self) {
        if mem::take(&mut self.needs_ty_option) {
            self.src
                .ts("export type Option<T> = { tag: 'none' } | { tag: 'some', val; T };\n");
        }
        if mem::take(&mut self.needs_ty_result) {
            self.src
                .ts("export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };\n");
        }
    }
}

impl<'a> InterfaceGenerator<'a> for JsInterface<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, _id: TypeId, name: &str, record: &Record, docs: &Docs) {
        self.docs(docs);
        self.src.ts(&format!(
            "export interface {} {{\n",
            name.to_upper_camel_case()
        ));
        for field in record.fields.iter() {
            self.docs(&field.docs);
            let (option_str, ty) = self
                .as_nullable(&field.ty)
                .map_or(("", &field.ty), |ty| ("?", ty));
            self.src.ts(&format!(
                "{}{}: ",
                field.name.to_lower_camel_case(),
                option_str
            ));
            self.print_ty(ty, Mode::Lift);
            self.src.ts(",\n");
        }
        self.src.ts("}\n");
    }

    fn type_tuple(&mut self, _id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_tuple(tuple, Mode::Lift);
        self.src.ts(";\n");
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.docs(docs);
        self.src.ts(&format!(
            "export interface {} {{\n",
            name.to_upper_camel_case()
        ));
        for flag in flags.flags.iter() {
            self.docs(&flag.docs);
            let name = flag.name.to_lower_camel_case();
            self.src.ts(&format!("{name}?: boolean,\n"));
        }
        self.src.ts("}\n");
    }

    fn type_variant(&mut self, _id: TypeId, name: &str, variant: &Variant, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        for (i, case) in variant.cases.iter().enumerate() {
            if i > 0 {
                self.src.ts(" | ");
            }
            self.src
                .ts(&format!("{}_{}", name, case.name).to_upper_camel_case());
        }
        self.src.ts(";\n");
        for case in variant.cases.iter() {
            self.docs(&case.docs);
            self.src.ts(&format!(
                "export interface {} {{\n",
                format!("{}_{}", name, case.name).to_upper_camel_case()
            ));
            self.src.ts("tag: '");
            self.src.ts(&case.name);
            self.src.ts("',\n");
            if let Some(ty) = case.ty {
                self.src.ts("val: ");
                self.print_ty(&ty, Mode::Lift);
                self.src.ts(",\n");
            }
            self.src.ts("}\n");
        }
    }

    fn type_union(&mut self, _id: TypeId, name: &str, union: &Union, docs: &Docs) {
        self.docs(docs);
        let name = name.to_upper_camel_case();
        self.src.ts(&format!("export type {name} = "));
        for i in 0..union.cases.len() {
            if i > 0 {
                self.src.ts(" | ");
            }
            self.src.ts(&format!("{name}{i}"));
        }
        self.src.ts(";\n");
        for (i, case) in union.cases.iter().enumerate() {
            self.docs(&case.docs);
            self.src.ts(&format!("export interface {name}{i} {{\n"));
            self.src.ts(&format!("tag: {i},\n"));
            self.src.ts("val: ");
            self.print_ty(&case.ty, Mode::Lift);
            self.src.ts(",\n");
            self.src.ts("}\n");
        }
    }

    fn type_option(&mut self, _id: TypeId, name: &str, payload: &Type, docs: &Docs) {
        self.docs(docs);
        let name = name.to_upper_camel_case();
        self.src.ts(&format!("export type {name} = "));
        if self.maybe_null(payload) {
            self.needs_ty_option = true;
            self.src.ts("Option<");
            self.print_ty(payload, Mode::Lift);
            self.src.ts(">");
        } else {
            self.print_ty(payload, Mode::Lift);
            self.src.ts(" | null");
        }
        self.src.ts(";\n");
    }

    fn type_result(&mut self, _id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.docs(docs);
        let name = name.to_upper_camel_case();
        self.needs_ty_result = true;
        self.src.ts(&format!("export type {name} = Result<"));
        self.print_optional_ty(result.ok.as_ref(), Mode::Lift);
        self.src.ts(", ");
        self.print_optional_ty(result.err.as_ref(), Mode::Lift);
        self.src.ts(">;\n");
    }

    fn type_enum(&mut self, _id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        // The complete documentation for this enum, including documentation for variants.
        let mut complete_docs = String::new();

        if let Some(docs) = &docs.contents {
            complete_docs.push_str(docs);
            // Add a gap before the `# Variants` section.
            complete_docs.push('\n');
        }

        writeln!(complete_docs, "# Variants").unwrap();

        for case in enum_.cases.iter() {
            writeln!(complete_docs).unwrap();
            writeln!(complete_docs, "## `\"{}\"`", case.name).unwrap();

            if let Some(docs) = &case.docs.contents {
                writeln!(complete_docs).unwrap();
                complete_docs.push_str(docs);
            }
        }

        self.docs_raw(&complete_docs);

        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        for (i, case) in enum_.cases.iter().enumerate() {
            if i != 0 {
                self.src.ts(" | ");
            }
            self.src.ts(&format!("'{}'", case.name));
        }
        self.src.ts(";\n");
    }

    fn type_alias(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_ty(ty, Mode::Lift);
        self.src.ts(";\n");
    }

    fn type_list(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_list(ty, Mode::Lift);
        self.src.ts(";\n");
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        drop((_id, name, ty, docs));
    }
}

#[derive(PartialEq)]
enum ErrHandling {
    None,
    ThrowResultErr,
    ResultCatchHandler,
}

struct FunctionBindgen<'a> {
    gen: &'a mut Js,
    sizes: SizeAlign,
    err: ErrHandling,
    tmp: usize,
    src: Source,
    block_storage: Vec<wit_bindgen_core::Source>,
    blocks: Vec<(String, Vec<String>)>,
    params: Vec<String>,
    memory: Option<String>,
    realloc: Option<String>,
    post_return: Option<String>,
    encoding: StringEncoding,
    callee: String,
}

impl FunctionBindgen<'_> {
    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn clamp_guest<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        let clamp = self.gen.intrinsic(Intrinsic::ClampGuest);
        results.push(format!("{}({}, {}, {})", clamp, operands[0], min, max));
    }

    fn load(&mut self, method: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        let memory = self.memory.as_ref().unwrap();
        let view = self.gen.intrinsic(Intrinsic::DataView);
        results.push(format!(
            "{view}({memory}).{method}({} + {offset}, true)",
            operands[0],
        ));
    }

    fn store(&mut self, method: &str, offset: i32, operands: &[String]) {
        let memory = self.memory.as_ref().unwrap();
        let view = self.gen.intrinsic(Intrinsic::DataView);
        uwriteln!(
            self.src.js,
            "{view}({memory}).{method}({} + {offset}, {}, true);",
            operands[1],
            operands[0]
        );
    }

    fn bind_results(&mut self, amt: usize, results: &mut Vec<String>) {
        match amt {
            0 => {}
            1 => {
                self.src.js("const ret = ");
                results.push("ret".to_string());
            }
            n => {
                self.src.js("const [");
                for i in 0..n {
                    if i > 0 {
                        self.src.js(", ");
                    }
                    self.src.js(&format!("ret{}", i));
                    results.push(format!("ret{}", i));
                }
                self.src.js("] = ");
            }
        }
    }
}

impl Bindgen for FunctionBindgen<'_> {
    type Operand = String;

    fn sizes(&self) -> &SizeAlign {
        &self.sizes
    }

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.src.js);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src.js, to_restore);
        self.blocks.push((src.into(), mem::take(operands)));
    }

    fn return_pointer(&mut self, _iface: &Interface, _size: usize, _align: usize) -> String {
        unimplemented!()
    }

    fn is_list_canonical(&self, iface: &Interface, ty: &Type) -> bool {
        self.gen.array_ty(iface, ty).is_some()
    }

    fn emit(
        &mut self,
        iface: &Interface,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(val.to_string()),
            Instruction::ConstZero { tys } => {
                for t in tys.iter() {
                    match t {
                        WasmType::I64 => results.push("0n".to_string()),
                        WasmType::I32 | WasmType::F32 | WasmType::F64 => {
                            results.push("0".to_string());
                        }
                    }
                }
            }

            // The representation of i32 in JS is a number, so 8/16-bit values
            // get further clamped to ensure that the upper bits aren't set when
            // we pass the value, ensuring that only the right number of bits
            // are transferred.
            Instruction::U8FromI32 => self.clamp_guest(results, operands, u8::MIN, u8::MAX),
            Instruction::S8FromI32 => self.clamp_guest(results, operands, i8::MIN, i8::MAX),
            Instruction::U16FromI32 => self.clamp_guest(results, operands, u16::MIN, u16::MAX),
            Instruction::S16FromI32 => self.clamp_guest(results, operands, i16::MIN, i16::MAX),
            // Use `>>>0` to ensure the bits of the number are treated as
            // unsigned.
            Instruction::U32FromI32 => {
                results.push(format!("{} >>> 0", operands[0]));
            }
            // All bigints coming from wasm are treated as signed, so convert
            // it to ensure it's treated as unsigned.
            Instruction::U64FromI64 => results.push(format!("BigInt.asUintN(64, {})", operands[0])),
            // Nothing to do signed->signed where the representations are the
            // same.
            Instruction::S32FromI32 | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap())
            }

            // All values coming from the host and going to wasm need to have
            // their ranges validated, since the host could give us any value.
            Instruction::I32FromU8 => {
                let conv = self.gen.intrinsic(Intrinsic::ToUint8);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I32FromS8 => {
                let conv = self.gen.intrinsic(Intrinsic::ToInt8);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I32FromU16 => {
                let conv = self.gen.intrinsic(Intrinsic::ToUint16);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I32FromS16 => {
                let conv = self.gen.intrinsic(Intrinsic::ToInt16);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I32FromU32 => {
                let conv = self.gen.intrinsic(Intrinsic::ToUint32);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I32FromS32 => {
                let conv = self.gen.intrinsic(Intrinsic::ToInt32);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I64FromU64 => {
                let conv = self.gen.intrinsic(Intrinsic::ToBigUint64);
                results.push(format!("{conv}({op})", op = operands[0]))
            }
            Instruction::I64FromS64 => {
                let conv = self.gen.intrinsic(Intrinsic::ToBigInt64);
                results.push(format!("{conv}({op})", op = operands[0]))
            }

            // The native representation in JS of f32 and f64 is just a number,
            // so there's nothing to do here. Everything wasm gives us is
            // representable in JS.
            Instruction::Float32FromF32 | Instruction::Float64FromF64 => {
                results.push(operands.pop().unwrap())
            }

            Instruction::F32FromFloat32 | Instruction::F64FromFloat64 => {
                // Use a unary `+` to cast to a float.
                results.push(format!("+{}", operands[0]));
            }

            // Validate that i32 values coming from wasm are indeed valid code
            // points.
            Instruction::CharFromI32 => {
                let validate = self.gen.intrinsic(Intrinsic::ValidateGuestChar);
                results.push(format!("{}({})", validate, operands[0]));
            }

            // Validate that strings are indeed 1 character long and valid
            // unicode.
            Instruction::I32FromChar => {
                let validate = self.gen.intrinsic(Intrinsic::ValidateHostChar);
                results.push(format!("{}({})", validate, operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                for (cast, op) in casts.iter().zip(operands) {
                    match cast {
                        Bitcast::I32ToF32 => {
                            let cvt = self.gen.intrinsic(Intrinsic::I32ToF32);
                            results.push(format!("{}({})", cvt, op));
                        }
                        Bitcast::F32ToI32 => {
                            let cvt = self.gen.intrinsic(Intrinsic::F32ToI32);
                            results.push(format!("{}({})", cvt, op));
                        }
                        Bitcast::I64ToF64 => {
                            let cvt = self.gen.intrinsic(Intrinsic::I64ToF64);
                            results.push(format!("{}({})", cvt, op));
                        }
                        Bitcast::F64ToI64 => {
                            let cvt = self.gen.intrinsic(Intrinsic::F64ToI64);
                            results.push(format!("{}({})", cvt, op));
                        }
                        Bitcast::I32ToI64 => results.push(format!("BigInt({})", op)),
                        Bitcast::I64ToI32 => results.push(format!("Number({})", op)),
                        Bitcast::I64ToF32 => {
                            let cvt = self.gen.intrinsic(Intrinsic::I32ToF32);
                            results.push(format!("{}(Number({}))", cvt, op));
                        }
                        Bitcast::F32ToI64 => {
                            let cvt = self.gen.intrinsic(Intrinsic::F32ToI32);
                            results.push(format!("BigInt({}({}))", cvt, op));
                        }
                        Bitcast::None => results.push(op.clone()),
                    }
                }
            }

            Instruction::BoolFromI32 => {
                let tmp = self.tmp();
                self.src
                    .js(&format!("const bool{} = {};\n", tmp, operands[0]));
                if self.gen.opts.valid_lifting_optimization {
                    results.push(format!("!!bool{tmp}"));
                } else {
                    let throw = self.gen.intrinsic(Intrinsic::ThrowInvalidBool);
                    results.push(format!(
                        "bool{tmp} == 0 ? false : (bool{tmp} == 1 ? true : {throw}())"
                    ));
                }
            }
            Instruction::I32FromBool => {
                results.push(format!("{} ? 1 : 0", operands[0]));
            }

            Instruction::RecordLower { record, .. } => {
                // use destructuring field access to get each
                // field individually.
                let tmp = self.tmp();
                let mut expr = "const {".to_string();
                for (i, field) in record.fields.iter().enumerate() {
                    if i > 0 {
                        expr.push_str(", ");
                    }
                    let name = format!("v{}_{}", tmp, i);
                    expr.push_str(&field.name.to_lower_camel_case());
                    expr.push_str(": ");
                    expr.push_str(&name);
                    results.push(name);
                }
                self.src.js(&format!("{} }} = {};\n", expr, operands[0]));
            }

            Instruction::RecordLift { record, .. } => {
                // records are represented as plain objects, so we
                // make a new object and set all the fields with an object
                // literal.
                let mut result = "{\n".to_string();
                for (field, op) in record.fields.iter().zip(operands) {
                    result.push_str(&format!("{}: {},\n", field.name.to_lower_camel_case(), op));
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::TupleLower { tuple, .. } => {
                // Tuples are represented as an array, sowe can use
                // destructuring assignment to lower the tuple into its
                // components.
                let tmp = self.tmp();
                let mut expr = "const [".to_string();
                for i in 0..tuple.types.len() {
                    if i > 0 {
                        expr.push_str(", ");
                    }
                    let name = format!("tuple{}_{}", tmp, i);
                    expr.push_str(&name);
                    results.push(name);
                }
                self.src.js(&format!("{}] = {};\n", expr, operands[0]));
            }

            Instruction::TupleLift { .. } => {
                // Tuples are represented as an array, so we just shove all
                // the operands into an array.
                results.push(format!("[{}]", operands.join(", ")));
            }

            // This lowers flags from a dictionary of booleans in accordance with https://webidl.spec.whatwg.org/#es-dictionary.
            Instruction::FlagsLower { flags, .. } => {
                let op0 = &operands[0];

                // Generate the result names.
                for _ in 0..flags.repr().count() {
                    let tmp = self.tmp();
                    let name = format!("flags{tmp}");
                    // Default to 0 so that in the null/undefined case, everything is false by
                    // default.
                    self.src.js(&format!("let {name} = 0;\n"));
                    results.push(name);
                }

                self.src.js(&format!(
                    "if (typeof {op0} === 'object' && {op0} !== null) {{\n"
                ));

                for (i, chunk) in flags.flags.chunks(32).enumerate() {
                    let result_name = &results[i];

                    self.src.js(&format!("{result_name} = "));
                    for (i, flag) in chunk.iter().enumerate() {
                        if i != 0 {
                            self.src.js(" | ");
                        }

                        let flag = flag.name.to_lower_camel_case();
                        self.src.js(&format!("Boolean({op0}.{flag}) << {i}"));
                    }
                    self.src.js(";\n");
                }

                self.src.js(&format!("\
                    }} else if ({op0} !== null && {op0} !== undefined) {{
                        throw new TypeError('only an object, undefined or null can be converted to flags');
                    }}
                "));

                // We don't need to do anything else for the null/undefined
                // case, since that's interpreted as everything false, and we
                // already defaulted everyting to 0.
            }

            Instruction::FlagsLift { flags, .. } => {
                let tmp = self.tmp();
                results.push(format!("flags{tmp}"));

                if let Some(op) = operands.last() {
                    // We only need an extraneous bits check if the number of flags isn't a multiple
                    // of 32, because if it is then all the bits are used and there are no
                    // extraneous bits.
                    if flags.flags.len() % 32 != 0 && !self.gen.opts.valid_lifting_optimization {
                        let mask: u32 = 0xffffffff << (flags.flags.len() % 32);
                        uwriteln!(
                            self.src.js,
                            "if (({op} & {mask}) !== 0) {{
                                throw new TypeError('flags have extraneous bits set');
                            }}"
                        );
                    }
                }

                uwriteln!(self.src.js, "const flags{tmp} = {{");

                for (i, flag) in flags.flags.iter().enumerate() {
                    let flag = flag.name.to_lower_camel_case();
                    let op = &operands[i / 32];
                    let mask: u32 = 1 << (i % 32);
                    uwriteln!(self.src.js, "{flag}: Boolean({op} & {mask}),");
                }

                uwriteln!(self.src.js, "}};");
            }

            Instruction::VariantPayloadName => results.push("e".to_string()),

            Instruction::VariantLower {
                variant,
                results: result_types,
                name,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let tmp = self.tmp();
                let operand = &operands[0];
                uwriteln!(self.src.js, "const variant{tmp} = {operand};");

                for i in 0..result_types.len() {
                    uwriteln!(self.src.js, "let variant{tmp}_{i};");
                    results.push(format!("variant{}_{}", tmp, i));
                }

                let expr_to_match = format!("variant{}.tag", tmp);

                uwriteln!(self.src.js, "switch ({expr_to_match}) {{");
                for (case, (block, block_results)) in variant.cases.iter().zip(blocks) {
                    uwriteln!(self.src.js, "case '{}': {{", case.name.as_str());
                    if case.ty.is_some() {
                        uwriteln!(self.src.js, "const e = variant{tmp}.val;");
                    }
                    self.src.js(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        uwriteln!(self.src.js, "variant{tmp}_{i} = {result};");
                    }
                    uwriteln!(
                        self.src.js,
                        "break;
                        }}"
                    );
                }
                let variant_name = name.to_upper_camel_case();
                uwriteln!(
                    self.src.js,
                    "default: {{
                        throw new TypeError('invalid variant specified for {variant_name}');
                    }}"
                );
                uwriteln!(self.src.js, "}}");
            }

            Instruction::VariantLift { variant, name, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let tmp = self.tmp();
                let operand = &operands[0];

                uwriteln!(
                    self.src.js,
                    "let variant{tmp};
                    switch ({operand}) {{"
                );

                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    let tag = case.name.as_str();
                    uwriteln!(
                        self.src.js,
                        "case {i}: {{
                            {block}\
                            variant{tmp} = {{
                                tag: '{tag}',"
                    );
                    if case.ty.is_some() {
                        assert!(block_results.len() == 1);
                        uwriteln!(self.src.js, "   val: {}", block_results[0]);
                    } else {
                        assert!(block_results.len() == 0);
                    }
                    uwriteln!(
                        self.src.js,
                        "   }};
                        break;
                        }}"
                    );
                }
                let variant_name = name.to_upper_camel_case();
                if !self.gen.opts.valid_lifting_optimization {
                    uwriteln!(
                        self.src.js,
                        "default: {{
                            throw new TypeError('invalid variant discriminant for {variant_name}');
                        }}"
                    );
                }
                uwriteln!(self.src.js, "}}");
                results.push(format!("variant{}", tmp));
            }

            Instruction::UnionLower {
                union,
                results: result_types,
                name,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                let tmp = self.tmp();
                let op0 = &operands[0];
                uwriteln!(self.src.js, "const union{tmp} = {op0};");

                for i in 0..result_types.len() {
                    uwriteln!(self.src.js, "let union{tmp}_{i};");
                    results.push(format!("union{tmp}_{i}"));
                }

                uwriteln!(self.src.js, "switch (union{tmp}.tag) {{");
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    uwriteln!(
                        self.src.js,
                        "case {i}: {{
                            const e = union{tmp}.val;
                            {block}"
                    );
                    for (i, result) in block_results.iter().enumerate() {
                        uwriteln!(self.src.js, "union{tmp}_{i} = {result};");
                    }
                    uwriteln!(
                        self.src.js,
                        "break;
                        }}"
                    );
                }
                let name = name.to_upper_camel_case();
                uwriteln!(
                    self.src.js,
                    "default: {{
                        throw new TypeError('invalid union specified for {name}');
                    }}"
                );
                uwriteln!(self.src.js, "}}");
            }

            Instruction::UnionLift { union, name, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let tmp = self.tmp();
                let operand = &operands[0];

                uwriteln!(
                    self.src.js,
                    "let union{tmp};
                    switch ({operand}) {{"
                );
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    assert!(block_results.len() == 1);
                    let block_result = &block_results[0];
                    uwriteln!(
                        self.src.js,
                        "case {i}: {{
                            {block}\
                            union{tmp} = {{
                                tag: {i},
                                val: {block_result},
                            }};
                            break;
                        }}"
                    );
                }
                let name = name.to_upper_camel_case();
                if !self.gen.opts.valid_lifting_optimization {
                    uwriteln!(
                        self.src.js,
                        "default: {{
                            throw new TypeError('invalid union discriminant for {name}');
                        }}"
                    );
                }
                uwriteln!(self.src.js, "}}");
                results.push(format!("union{tmp}"));
            }

            Instruction::OptionLower {
                payload,
                results: result_types,
                ..
            } => {
                let (mut some, some_results) = self.blocks.pop().unwrap();
                let (mut none, none_results) = self.blocks.pop().unwrap();

                let tmp = self.tmp();
                let operand = &operands[0];
                uwriteln!(self.src.js, "const variant{tmp} = {operand};");

                for i in 0..result_types.len() {
                    uwriteln!(self.src.js, "let variant{tmp}_{i};");
                    results.push(format!("variant{tmp}_{i}"));

                    let some_result = &some_results[i];
                    let none_result = &none_results[i];
                    uwriteln!(some, "variant{tmp}_{i} = {some_result};");
                    uwriteln!(none, "variant{tmp}_{i} = {none_result};");
                }

                if self.gen.maybe_null(iface, payload) {
                    uwriteln!(
                        self.src.js,
                        "switch (variant{tmp}.tag) {{
                            case 'none': {{
                                {none}\
                                break;
                            }}
                            case 'some': {{
                                const e = variant{tmp}.val;
                                {some}\
                                break;
                            }}
                            default: {{
                                throw new TypeError('invalid variant specified for option');
                            }}
                        }}"
                    );
                } else {
                    uwriteln!(
                        self.src.js,
                        "if (variant{tmp} === null || variant{tmp} === undefined) {{
                            {none}\
                        }} else {{
                            const e = variant{tmp};
                            {some}\
                        }}"
                    );
                }
            }

            Instruction::OptionLift { payload, .. } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.len() == 0);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];

                let tmp = self.tmp();
                let operand = &operands[0];

                let (v_none, v_some) = if self.gen.maybe_null(iface, payload) {
                    (
                        "{ tag: 'none' }",
                        format!(
                            "{{
                                tag: 'some',
                                val: {some_result}
                            }}"
                        ),
                    )
                } else {
                    ("null", some_result.into())
                };

                if !self.gen.opts.valid_lifting_optimization {
                    uwriteln!(
                        self.src.js,
                        "let variant{tmp};
                        switch ({operand}) {{
                            case 0: {{
                                {none}\
                                variant{tmp} = {v_none};
                                break;
                            }}
                            case 1: {{
                                {some}\
                                variant{tmp} = {v_some};
                                break;
                            }}
                            default: {{
                                throw new TypeError('invalid variant discriminant for option');
                            }}
                        }}"
                    );
                } else {
                    uwriteln!(
                        self.src.js,
                        "let variant{tmp};
                        if ({operand}) {{
                            {some}\
                            variant{tmp} = {v_some};
                        }} else {{
                            {none}\
                            variant{tmp} = {v_none};
                        }}"
                    );
                }

                results.push(format!("variant{tmp}"));
            }

            Instruction::ResultLower {
                results: result_types,
                ..
            } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                let (mut ok, ok_results) = self.blocks.pop().unwrap();

                let tmp = self.tmp();
                let operand = &operands[0];
                uwriteln!(self.src.js, "const variant{tmp} = {operand};");

                for i in 0..result_types.len() {
                    uwriteln!(self.src.js, "let variant{tmp}_{i};");
                    results.push(format!("variant{tmp}_{i}"));

                    let ok_result = &ok_results[i];
                    let err_result = &err_results[i];
                    uwriteln!(ok, "variant{tmp}_{i} = {ok_result};");
                    uwriteln!(err, "variant{tmp}_{i} = {err_result};");
                }

                uwriteln!(
                    self.src.js,
                    "switch (variant{tmp}.tag) {{
                        case 'ok': {{
                            const e = variant{tmp}.val;
                            {ok}\
                            break;
                        }}
                        case 'err': {{
                            const e = variant{tmp}.val;
                            {err}\
                            break;
                        }}
                        default: {{
                            throw new TypeError('invalid variant specified for result');
                        }}
                    }}"
                );
            }

            Instruction::ResultLift { result, .. } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let ok_result = if result.ok.is_some() {
                    assert_eq!(ok_results.len(), 1);
                    format!("{}", ok_results[0])
                } else {
                    assert_eq!(ok_results.len(), 0);
                    String::from("undefined")
                };
                let err_result = if result.err.is_some() {
                    assert_eq!(err_results.len(), 1);
                    format!("{}", err_results[0])
                } else {
                    assert_eq!(err_results.len(), 0);
                    String::from("undefined")
                };
                let tmp = self.tmp();
                let op0 = &operands[0];

                if !self.gen.opts.valid_lifting_optimization {
                    uwriteln!(
                        self.src.js,
                        "let variant{tmp};
                        switch ({op0}) {{
                            case 0: {{
                                {ok}\
                                variant{tmp} = {{
                                    tag: 'ok',
                                    val: {ok_result}
                                }};
                                break;
                            }}
                            case 1: {{
                                {err}\
                                variant{tmp} = {{
                                    tag: 'err',
                                    val: {err_result}
                                }};
                                break;
                            }}
                            default: {{
                                throw new TypeError('invalid variant discriminant for expected');
                            }}
                        }}"
                    );
                } else {
                    uwriteln!(
                        self.src.js,
                        "let variant{tmp};
                        if ({op0}) {{
                            {err}\
                            variant{tmp} = {{
                                tag: 'err',
                                val: {err_result}
                            }};
                        }} else {{
                            {ok}\
                            variant{tmp} = {{
                                tag: 'ok',
                                val: {ok_result}
                            }};
                        }}"
                    );
                }
                results.push(format!("variant{tmp}"));
            }

            // Lowers an enum in accordance with https://webidl.spec.whatwg.org/#es-enumeration.
            Instruction::EnumLower { name, enum_, .. } => {
                let tmp = self.tmp();

                let to_string = self.gen.intrinsic(Intrinsic::ToString);
                let operand = &operands[0];
                uwriteln!(self.src.js, "const val{tmp} = {to_string}({operand});");

                // Declare a variable to hold the result.
                uwriteln!(
                    self.src.js,
                    "let enum{tmp};
                    switch (val{tmp}) {{"
                );
                for (i, case) in enum_.cases.iter().enumerate() {
                    uwriteln!(
                        self.src.js,
                        "case '{case}': {{
                            enum{tmp} = {i};
                            break;
                        }}",
                        case = case.name
                    );
                }
                uwriteln!(
                    self.src.js,
                    "default: {{
                        throw new TypeError(`\"${{val{tmp}}}\" is not one of the cases of {name}`);
                    }}"
                );
                uwriteln!(self.src.js, "}}");

                results.push(format!("enum{tmp}"));
            }

            Instruction::EnumLift { name, enum_, .. } => {
                let tmp = self.tmp();

                uwriteln!(
                    self.src.js,
                    "let enum{tmp};
                    switch ({}) {{",
                    operands[0]
                );
                for (i, case) in enum_.cases.iter().enumerate() {
                    uwriteln!(
                        self.src.js,
                        "case {i}: {{
                            enum{tmp} = '{case}';
                            break;
                        }}",
                        case = case.name
                    );
                }
                if !self.gen.opts.valid_lifting_optimization {
                    let name = name.to_upper_camel_case();
                    uwriteln!(
                        self.src.js,
                        "default: {{
                            throw new TypeError('invalid discriminant specified for {name}');
                        }}",
                    );
                }
                uwriteln!(self.src.js, "}}");

                results.push(format!("enum{tmp}"));
            }

            Instruction::ListCanonLower { element, .. } => {
                let tmp = self.tmp();
                let memory = self.memory.as_ref().unwrap();
                let realloc = self.realloc.as_ref().unwrap();

                let size = self.sizes.size(element);
                let align = self.sizes.align(element);
                uwriteln!(self.src.js, "const val{tmp} = {};", operands[0]);
                if matches!(element, Type::U8) {
                    uwriteln!(self.src.js, "const len{tmp} = val{tmp}.byteLength;");
                } else {
                    uwriteln!(self.src.js, "const len{tmp} = val{tmp}.length;");
                }
                uwriteln!(
                    self.src.js,
                    "const ptr{tmp} = {realloc}(0, 0, {align}, len{tmp} * {size});"
                );
                // TODO: this is the wrong endianness
                if matches!(element, Type::U8) {
                    uwriteln!(
                        self.src.js,
                        "const src{tmp} = new Uint8Array(val{tmp}.buffer || val{tmp}, val{tmp}.byteOffset, len{tmp} * {size});",
                    );
                } else {
                    uwriteln!(
                        self.src.js,
                        "const src{tmp} = new Uint8Array(val{tmp}.buffer, val{tmp}.byteOffset, len{tmp} * {size});",
                    );
                }
                uwriteln!(
                    self.src.js,
                    "(new Uint8Array({memory}.buffer, ptr{tmp}, len{tmp} * {size})).set(src{tmp});",
                );
                results.push(format!("ptr{}", tmp));
                results.push(format!("len{}", tmp));
            }
            Instruction::ListCanonLift { element, .. } => {
                let tmp = self.tmp();
                let memory = self.memory.as_ref().unwrap();
                uwriteln!(self.src.js, "const ptr{tmp} = {};", operands[0]);
                uwriteln!(self.src.js, "const len{tmp} = {};", operands[1]);
                // TODO: this is the wrong endianness
                let array_ty = self.gen.array_ty(iface, element).unwrap();
                uwriteln!(
                    self.src.js,
                    "const result{tmp} = new {array_ty}({memory}.buffer.slice(ptr{tmp}, ptr{tmp} + len{tmp} * {}));",
                    self.sizes.size(element),
                );
                results.push(format!("result{tmp}"));
            }
            Instruction::StringLower { .. } => {
                // Only Utf8 and Utf16 supported for now
                assert!(matches!(
                    self.encoding,
                    StringEncoding::Utf8 | StringEncoding::Utf16
                ));
                let tmp = self.tmp();
                let memory = self.memory.as_ref().unwrap();
                let realloc = self.realloc.as_ref().unwrap();

                let intrinsic = if self.encoding == StringEncoding::Utf16 {
                    Intrinsic::Utf16Encode
                } else {
                    Intrinsic::Utf8Encode
                };
                let encode = self.gen.intrinsic(intrinsic);
                uwriteln!(
                    self.src.js,
                    "const ptr{tmp} = {encode}({}, {realloc}, {memory});",
                    operands[0],
                );
                if self.encoding == StringEncoding::Utf8 {
                    let encoded_len = self.gen.intrinsic(Intrinsic::Utf8EncodedLen);
                    uwriteln!(self.src.js, "const len{tmp} = {encoded_len};");
                } else {
                    uwriteln!(self.src.js, "const len{tmp} = {}.length;", operands[0]);
                }
                results.push(format!("ptr{}", tmp));
                results.push(format!("len{}", tmp));
            }
            Instruction::StringLift => {
                // Only Utf8 and Utf16 supported for now
                assert!(matches!(
                    self.encoding,
                    StringEncoding::Utf8 | StringEncoding::Utf16
                ));
                let tmp = self.tmp();
                let memory = self.memory.as_ref().unwrap();
                uwriteln!(self.src.js, "const ptr{tmp} = {};", operands[0]);
                uwriteln!(self.src.js, "const len{tmp} = {};", operands[1]);
                let intrinsic = if self.encoding == StringEncoding::Utf16 {
                    Intrinsic::Utf16Decoder
                } else {
                    Intrinsic::Utf8Decoder
                };
                let decoder = self.gen.intrinsic(intrinsic);
                uwriteln!(
                    self.src.js,
                    "const result{tmp} = {decoder}.decode(new Uint{}Array({memory}.buffer, ptr{tmp}, len{tmp}));",
                    if self.encoding == StringEncoding::Utf16 { "16" } else { "8" }
                );
                results.push(format!("result{tmp}"));
            }

            Instruction::ListLower { element, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                assert!(body_results.is_empty());
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                let size = self.sizes.size(element);
                let align = self.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                uwriteln!(self.src.js, "const {vec} = {};", operands[0]);
                uwriteln!(self.src.js, "const {len} = {vec}.length;");

                // ... then realloc space for the result in the guest module
                let realloc = self.realloc.as_ref().unwrap();
                uwriteln!(
                    self.src.js,
                    "const {result} = {realloc}(0, 0, {align}, {len} * {size});"
                );

                // ... then consume the vector and use the block to lower the
                // result.
                uwriteln!(self.src.js, "for (let i = 0; i < {vec}.length; i++) {{");
                uwriteln!(self.src.js, "const e = {vec}[i];");
                uwrite!(self.src.js, "const base = {result} + i * {size};");
                self.src.js(&body);
                self.src.js("}\n");

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.sizes.size(element);
                let len = format!("len{tmp}");
                uwriteln!(self.src.js, "const {len} = {};", operands[1]);
                let base = format!("base{tmp}");
                uwriteln!(self.src.js, "const {base} = {};", operands[0]);
                let result = format!("result{tmp}");
                uwriteln!(self.src.js, "const {result} = [];");
                results.push(result.clone());

                uwriteln!(self.src.js, "for (let i = 0; i < {len}; i++) {{");
                uwriteln!(self.src.js, "const base = {base} + i * {size};");
                self.src.js(&body);
                assert_eq!(body_results.len(), 1);
                uwriteln!(self.src.js, "{result}.push({});", body_results[0]);
                self.src.js("}\n");
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm { sig, .. } => {
                self.bind_results(sig.results.len(), results);
                uwriteln!(self.src.js, "{}({});", self.callee, operands.join(", "));
            }

            Instruction::CallInterface { func } => {
                if self.err == ErrHandling::ResultCatchHandler {
                    uwriteln!(
                        self.src.js,
                        "let ret;
                        try {{
                            ret = {{ tag: 'ok', val: {}({}) }};
                        }} catch (e) {{
                            ret = {{ tag: 'err', val: {}(e) }};
                        }}",
                        self.callee,
                        operands.join(", "),
                        self.gen.intrinsic(Intrinsic::GetErrorPayload),
                    );
                    results.push("ret".to_string());
                } else {
                    self.bind_results(func.results.len(), results);
                    uwriteln!(self.src.js, "{}({});", self.callee, operands.join(", "));
                }
            }

            Instruction::Return { amt, .. } => {
                if let Some(f) = &self.post_return {
                    uwriteln!(self.src.js, "{f}(ret);");
                }

                if self.err == ErrHandling::ThrowResultErr {
                    let component_err = self.gen.intrinsic(Intrinsic::ComponentError);
                    let operand = &operands[0];
                    uwriteln!(
                        self.src.js,
                        "if ({operand}.tag === 'err') {{
                            throw new {component_err}({operand}.val);
                        }}
                        return {operand}.val;"
                    );
                } else {
                    match amt {
                        0 => {}
                        1 => uwriteln!(self.src.js, "return {};", operands[0]),
                        _ => uwriteln!(self.src.js, "return [{}];", operands.join(", ")),
                    }
                }
            }

            Instruction::I32Load { offset } => self.load("getInt32", *offset, operands, results),
            Instruction::I64Load { offset } => self.load("getBigInt64", *offset, operands, results),
            Instruction::F32Load { offset } => self.load("getFloat32", *offset, operands, results),
            Instruction::F64Load { offset } => self.load("getFloat64", *offset, operands, results),
            Instruction::I32Load8U { offset } => self.load("getUint8", *offset, operands, results),
            Instruction::I32Load8S { offset } => self.load("getInt8", *offset, operands, results),
            Instruction::I32Load16U { offset } => {
                self.load("getUint16", *offset, operands, results)
            }
            Instruction::I32Load16S { offset } => self.load("getInt16", *offset, operands, results),
            Instruction::I32Store { offset } => self.store("setInt32", *offset, operands),
            Instruction::I64Store { offset } => self.store("setBigInt64", *offset, operands),
            Instruction::F32Store { offset } => self.store("setFloat32", *offset, operands),
            Instruction::F64Store { offset } => self.store("setFloat64", *offset, operands),
            Instruction::I32Store8 { offset } => self.store("setInt8", *offset, operands),
            Instruction::I32Store16 { offset } => self.store("setInt16", *offset, operands),

            Instruction::Malloc { size, align, .. } => {
                let tmp = self.tmp();
                let realloc = self.realloc.as_ref().unwrap();
                let ptr = format!("ptr{tmp}");
                uwriteln!(
                    self.src.js,
                    "const {ptr} = {realloc}(0, 0, {align}, {size});",
                );
                results.push(ptr);
            }

            i => unimplemented!("{:?}", i),
        }
    }
}

fn to_js_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "import" => "import_",
        s => s,
    }
}

#[cfg(feature = "clap")]
fn maps_str_to_map(maps: &str) -> Result<HashMap<String, String>> {
    let mut map_hash = HashMap::<String, String>::new();
    for mapping in maps.split(",") {
        match mapping.split_once('=') {
            Some((left, right)) => {
                map_hash.insert(left.into(), right.into());
            }
            None => return Err(anyhow!(format!("Invalid mapping entry \"{}\"", &mapping))),
        };
    }
    Ok(map_hash)
}

// https://tc39.es/ecma262/#prod-IdentifierStartChar
// Unicode ID_Start | "$" | "_"
fn is_js_identifier_start(code: char) -> bool {
    return match code {
        'A'..='Z' | 'a'..='z' | '$' | '_' => true,
        // leaving out non-ascii for now...
        _ => false,
    };
}

// https://tc39.es/ecma262/#prod-IdentifierPartChar
// Unicode ID_Continue | "$" | U+200C | U+200D
fn is_js_identifier_char(code: char) -> bool {
    return match code {
        '0'..='9' | 'A'..='Z' | 'a'..='z' | '$' | '_' => true,
        // leaving out non-ascii for now...
        _ => false,
    };
}

fn is_js_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(char) = chars.next() {
        if !is_js_identifier_start(char) {
            return false;
        }
    } else {
        return false;
    }
    while let Some(char) = chars.next() {
        if !is_js_identifier_char(char) {
            return false;
        }
    }
    return true;
}

#[derive(Default)]
struct Source {
    js: wit_bindgen_core::Source,
    js_intrinsics: wit_bindgen_core::Source,
    js_init: wit_bindgen_core::Source,
    ts: wit_bindgen_core::Source,
}

impl Source {
    fn js(&mut self, s: &str) {
        self.js.push_str(s);
    }
    fn js_intrinsics(&mut self, s: &str) {
        self.js_intrinsics.push_str(s);
    }
    fn js_init(&mut self, s: &str) {
        self.js_init.push_str(s);
    }
    fn ts(&mut self, s: &str) {
        self.ts.push_str(s);
    }
}
