use heck::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};
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
use wit_component::ComponentInterfaces;

#[derive(Default)]
struct Js {
    /// The source code for the "main" file that's going to be created for the
    /// component we're generating bindings for. This is incrementally added to
    /// over time and primarily contains the main `instantiate` function as well
    /// as a type-description of the input/output interfaces.
    src: Source,

    /// Type script definitions which will become the import object
    import_object: wit_bindgen_core::Source,
    /// Type script definitions which will become the export object
    export_object: wit_bindgen_core::Source,

    /// Various options for code generation.
    opts: Opts,

    /// List of all intrinsics emitted to `src` so far.
    all_intrinsics: BTreeSet<Intrinsic>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Disables generation of `*.d.ts` files and instead only generates `*.js`
    /// source files.
    #[cfg_attr(feature = "clap", arg(long = "no-typescript"))]
    pub no_typescript: bool,
}

impl Opts {
    pub fn build(self) -> Box<dyn ComponentGenerator> {
        let mut gen = Js::default();
        gen.opts = self;
        Box::new(gen)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum Intrinsic {
    ClampGuest,
    DataView,
    ValidateGuestChar,
    ValidateHostChar,
    IsLE,
    /// Implementation of https://tc39.es/ecma262/#sec-toint32.
    ToInt32,
    /// Implementation of https://tc39.es/ecma262/#sec-touint32.
    ToUint32,
    /// Implementation of https://tc39.es/ecma262/#sec-toint16.
    ToInt16,
    /// Implementation of https://tc39.es/ecma262/#sec-touint16.
    ToUint16,
    /// Implementation of https://tc39.es/ecma262/#sec-toint8.
    ToInt8,
    /// Implementation of https://tc39.es/ecma262/#sec-touint8.
    ToUint8,
    /// Implementation of https://tc39.es/ecma262/#sec-tobigint64.
    ToBigInt64,
    /// Implementation of https://tc39.es/ecma262/#sec-tobiguint64.
    ToBigUint64,
    /// Implementation of https://tc39.es/ecma262/#sec-tostring.
    ToString,
    I32ToF32,
    F32ToI32,
    I64ToF64,
    F64ToI64,
    Utf8Decoder,
    Utf16Decoder,
    Utf8Encode,
    Utf16Encode,
    Utf8EncodedLen,
    ThrowInvalidBool,
}

impl Intrinsic {
    fn name(&self) -> &'static str {
        match self {
            Intrinsic::ClampGuest => "clampGuest",
            Intrinsic::DataView => "dataView",
            Intrinsic::ValidateGuestChar => "validateGuestChar",
            Intrinsic::ValidateHostChar => "validateHostChar",
            Intrinsic::IsLE => "isLE",
            Intrinsic::ToInt32 => "toInt32",
            Intrinsic::ToUint32 => "toUint32",
            Intrinsic::ToInt16 => "toInt16",
            Intrinsic::ToUint16 => "toUint16",
            Intrinsic::ToInt8 => "toInt8",
            Intrinsic::ToUint8 => "toUint8",
            Intrinsic::ToBigInt64 => "toInt64",
            Intrinsic::ToBigUint64 => "toUint64",
            Intrinsic::ToString => "toString",
            Intrinsic::F32ToI32 => "f32ToI32",
            Intrinsic::I32ToF32 => "i32ToF32",
            Intrinsic::F64ToI64 => "f64ToI64",
            Intrinsic::I64ToF64 => "i64ToF64",
            Intrinsic::Utf8Decoder => "utf8Decoder",
            Intrinsic::Utf16Decoder => "utf16Decoder",
            Intrinsic::Utf8Encode => "utf8Encode",
            Intrinsic::Utf16Encode => "utf16Encode",
            Intrinsic::Utf8EncodedLen => "utf8EncodedLen",
            Intrinsic::ThrowInvalidBool => "throwInvalidBool",
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
        name: &str,
        component: &Component,
        modules: &PrimaryMap<StaticModuleIndex, ModuleTranslation<'_>>,
        interfaces: &ComponentInterfaces,
    ) {
        // Generate the TypeScript definition of the `instantiate` function
        // which is the main workhorse of the generated bindings.
        let camel = name.to_upper_camel_case();
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
                * The first argument to this function, `instantiateCore`, is
                * used to instantiate core wasm modules within the component.
                * Components are composed of core wasm modules and this callback
                * will be invoked per core wasm instantiation. The caller of
                * this function is responsible for reading the core wasm module
                * identified by `path` and instantiating it with the core wasm
                * import object provided. This would use `instantiateStreaming`
                * on the web, for example.
                */
                export function instantiate(
                    instantiateCore: (path: string, imports: any) => Promise<WebAssembly.Instance>,
                    imports: ImportObject,
                ): Promise<{camel}>;
            ",
        );

        // bindings is the actual `instantiate` method itself, created by this
        // structure.
        let mut instantiator = Instantiator {
            src: Source::default(),
            gen: self,
            modules,
            instances: Default::default(),
            interfaces,
            component,
        };
        instantiator.instantiate();
        instantiator.gen.src.js(&instantiator.src.js);
        assert!(instantiator.src.ts.is_empty());
    }

    fn finish_component(&mut self, name: &str, files: &mut Files) {
        files.push(&format!("{name}.js"), self.src.js.as_bytes());
        if !self.opts.no_typescript {
            files.push(&format!("{name}.d.ts"), self.src.ts.as_bytes());
        }
    }
}

impl WorldGenerator for Js {
    fn import(&mut self, name: &str, iface: &Interface, files: &mut Files) {
        self.generate_interface(name, iface, "imports", "Imports", files);
        let camel = name.to_upper_camel_case();
        uwriteln!(self.import_object, "{name}: {camel}Imports;");
    }

    fn export(&mut self, name: &str, iface: &Interface, files: &mut Files) {
        self.generate_interface(name, iface, "exports", "Exports", files);
        let camel = name.to_upper_camel_case();
        uwriteln!(self.export_object, "{name}: {camel}Exports;");
    }

    fn export_default(&mut self, _name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = self.js_interface(iface);
        for func in iface.functions.iter() {
            gen.ts_func(func);
        }
        gen.gen.export_object.push_str(&mem::take(&mut gen.src.ts));

        // After the default interface has its function definitions
        // inlined the rest of the types are generated here as well.
        gen.types();
        gen.post_types();
        gen.gen.src.ts(&mem::take(&mut gen.src.ts));
    }

    fn finish(&mut self, name: &str, _interfaces: &ComponentInterfaces, _files: &mut Files) {
        let camel = name.to_upper_camel_case();

        // Generate a type definition for the import object to type-check
        // all imports to the component.
        //
        // With the current representation of a "world" this is an import object
        // per-imported-interface where the type of that field is defined by the
        // interface itself.
        uwriteln!(self.src.ts, "export interface ImportObject {{");
        self.src.ts(&self.import_object);
        uwriteln!(self.src.ts, "}}");

        // Generate a type definition for the export object from instantiating
        // the component.
        uwriteln!(self.src.ts, "export interface {camel} {{",);
        self.src.ts(&self.export_object);
        uwriteln!(self.src.ts, "}}");
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
    ) {
        let camel = name.to_upper_camel_case();
        let mut gen = self.js_interface(iface);
        gen.types();
        gen.post_types();

        uwriteln!(gen.src.ts, "export interface {camel} {{");
        for func in iface.functions.iter() {
            gen.ts_func(func);
        }
        uwriteln!(gen.src.ts, "}}");

        assert!(gen.src.js.is_empty());
        if !gen.gen.opts.no_typescript {
            files.push(&format!("{dir}/{name}.d.ts"), gen.src.ts.as_bytes());
        }

        uwriteln!(
            self.src.ts,
            "import {{ {camel} as {camel}{extra} }} from \"./{dir}/{name}\";"
        );
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
            self.src.js("
                const i32ToF32I = new Int32Array(1);
                const i32ToF32F = new Float32Array(i32ToF32I.buffer);
            ");
        }
        if (i == Intrinsic::I64ToF64 && !self.all_intrinsics.contains(&Intrinsic::F64ToI64))
            || (i == Intrinsic::F64ToI64 && !self.all_intrinsics.contains(&Intrinsic::I64ToF64))
        {
            self.src.js("
                const i64ToF64I = new BigInt64Array(1);
                const i64ToF64F = new Float64Array(i64ToF64I.buffer);
            ");
        }

        match i {
            Intrinsic::ClampGuest => self.src.js("
                function clampGuest(i, min, max) {
                    if (i < min || i > max) \
                        throw new RangeError(`must be between ${min} and ${max}`);
                    return i;
                }
            "),

            Intrinsic::DataView => self.src.js("
                let dv = new DataView(new ArrayBuffer());
                const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);
            "),

            Intrinsic::IsLE => self.src.js("
                const isLE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
            "),

            Intrinsic::ValidateGuestChar => self.src.js("
                function validateGuestChar(i) {
                    if ((i > 0x10ffff) || (i >= 0xd800 && i <= 0xdfff)) \
                        throw new RangeError(`not a valid char`);
                    return String.fromCodePoint(i);
                }
            "),

            // TODO: this is incorrect. It at least allows strings of length > 0
            // but it probably doesn't do the right thing for unicode or invalid
            // utf16 strings either.
            Intrinsic::ValidateHostChar => self.src.js("
                function validateHostChar(s) {
                    if (typeof s !== 'string') \
                        throw new TypeError(`must be a string`);
                    return s.codePointAt(0);
                }
            "),


            Intrinsic::ToInt32 => self.src.js("
                function toInt32(val) {
                    return val >> 0;
                }
            "),
            Intrinsic::ToUint32 => self.src.js("
                function toUint32(val) {
                    return val >>> 0;
                }
            "),

            Intrinsic::ToInt16 => self.src.js("
                function toInt16(val) {
                    val >>>= 0;
                    val %= 2 ** 16;
                    if (val >= 2 ** 15) {
                        val -= 2 ** 16;
                    }
                    return val;
                }
            "),
            Intrinsic::ToUint16 => self.src.js("
                function toUint16(val) {
                    val >>>= 0;
                    val %= 2 ** 16;
                    return val;
                }
            "),
            Intrinsic::ToInt8 => self.src.js("
                function toInt8(val) {
                    val >>>= 0;
                    val %= 2 ** 8;
                    if (val >= 2 ** 7) {
                        val -= 2 ** 8;
                    }
                    return val;
                }
            "),
            Intrinsic::ToUint8 => self.src.js("
                function toUint8(val) {
                    val >>>= 0;
                    val %= 2 ** 8;
                    return val;
                }
            "),

            Intrinsic::ToBigInt64 => self.src.js("
                function toInt64(val) {
                    return BigInt.asIntN(64, val);
                }
            "),
            Intrinsic::ToBigUint64 => self.src.js("
                function toUint64(val) {
                    return BigInt.asUintN(64, val);
                }
            "),

            Intrinsic::ToString => self.src.js("
                function toString(val) {
                    if (typeof val === 'symbol') {
                        throw new TypeError('symbols cannot be converted to strings');
                    } else {
                        // Calling `String` almost directly calls `ToString`, except that it also allows symbols,
                        // which is why we have the symbol-rejecting branch above.
                        //
                        // Definition of `String`: https://tc39.es/ecma262/#sec-string-constructor-string-value
                        return String(val);
                    }
                }
            "),

            Intrinsic::I32ToF32 => self.src.js("
                function i32ToF32(i) {
                    i32ToF32I[0] = i;
                    return i32ToF32F[0];
                }
            "),
            Intrinsic::F32ToI32 => self.src.js("
                function f32ToI32(f) {
                    i32ToF32F[0] = f;
                    return i32ToF32I[0];
                }
            "),
            Intrinsic::I64ToF64 => self.src.js("
                function i64ToF64(i) {
                    i64ToF64I[0] = i;
                    return i64ToF64F[0];
                }
            "),
            Intrinsic::F64ToI64 => self.src.js("
                function f64ToI64(f) {
                    i64ToF64F[0] = f;
                    return i64ToF64I[0];
                }
            "),

            Intrinsic::Utf8Decoder => self
                .src
                .js("const utf8Decoder = new TextDecoder();\n"),

            Intrinsic::Utf16Decoder => self
                .src
                .js("const utf16Decoder = new TextDecoder('utf-16');\n"),

            Intrinsic::Utf8EncodedLen => self.src.js("let utf8EncodedLen = 0;\n"),

            Intrinsic::Utf8Encode => self.src.js("
                const utf8Encoder = new TextEncoder();

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

            Intrinsic::Utf16Encode => self.src.js("
                function utf16Encode (str, realloc, memory) {
                    const len = str.length, ptr = realloc(0, 0, 2, len), out = new Uint16Array(memory.buffer, ptr, len);
                    let i = 0;
                    if (isLE) {
                        while (i < len) out[i] = str.charCodeAt(i++);
                    } else {
                        while (i < len) {
                            const ch = str.charCodeAt(i);
                            out[i++] = (ch & 0xff) << 8 | ch >>> 8;
                        }
                    }
                    return ptr;
                }
            "),

            Intrinsic::ThrowInvalidBool => self.src.js("
                function throwInvalidBool() {
                    throw new RangeError(\"invalid variant discriminant for bool\");
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
    interfaces: &'a ComponentInterfaces,
    component: &'a Component,
}

impl Instantiator<'_> {
    fn instantiate(&mut self) {
        uwriteln!(
            self.src.js,
            "export async function instantiate(instantiateCore, imports) {{"
        );

        for init in self.component.initializers.iter() {
            self.global_initializer(init);
        }

        self.src.js("return ");
        self.exports(&self.component.exports, 0, self.interfaces.default.as_ref());
        self.src.js(";\n");

        uwriteln!(self.src.js, "}}");
    }

    fn global_initializer(&mut self, init: &GlobalInitializer) {
        match init {
            GlobalInitializer::InstantiateModule(m) => match m {
                InstantiateModule::Static(idx, args) => self.instantiate_static_module(*idx, args),

                // This is only needed when instantiating an imported core wasm
                // module which while easy to implement here is not possible to
                // test at this time so it's left unimplemented.
                InstantiateModule::Import(..) => unimplemented!(),
            },

            GlobalInitializer::LowerImport(i) => self.lower_import(i),

            GlobalInitializer::ExtractMemory(m) => {
                let def = self.core_export(&m.export);
                uwriteln!(self.src.js, "const memory{} = {def};", m.index.as_u32());
            }
            GlobalInitializer::ExtractRealloc(r) => {
                let def = self.core_def(&r.def);
                uwriteln!(self.src.js, "const realloc{} = {def};", r.index.as_u32());
            }
            GlobalInitializer::ExtractPostReturn(p) => {
                let def = self.core_def(&p.def);
                uwriteln!(self.src.js, "const postReturn{} = {def};", p.index.as_u32());
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
        if import_obj.is_empty() {
            imports.push_str("{}");
        } else {
            imports.push_str("{\n");
            for (module, names) in import_obj {
                uwrite!(imports, "\"{module}\": {{\n");
                for (name, val) in names {
                    uwriteln!(imports, "\"{name}\": {val},");
                }
                imports.push_str("},\n");
            }
            imports.push_str("}");
        }

        // Delegate most of the work to `instantiateCore` to allow the JS caller
        // to do `instantiateStreaming` or w/e is appropriate for the embedding
        // at hand. We've done all the hard work of assembling the import object
        // so the instantiation should be relatively straightforward.
        let i = self.instances.push(idx);
        let name = format!("module{}.wasm", idx.as_u32());
        uwrite!(self.src.js, "const instance{} = ", i.as_u32());
        uwriteln!(self.src.js, "await instantiateCore(\"{name}\", {imports});");
    }

    fn lower_import(&mut self, import: &LowerImport) {
        // Determine the `Interface` that this import corresponds to. At this
        // time `wit-component` only supports root-level imports of instances
        // where instances export functions.
        let (import_index, path) = &self.component.imports[import.import];
        let (import_name, _import_ty) = &self.component.import_types[*import_index];
        assert_eq!(path.len(), 1);
        let iface = &self.interfaces.imports[import_name.as_str()];
        let func = iface.functions.iter().find(|f| f.name == path[0]).unwrap();

        let index = import.index.as_u32();
        let callee = format!("lowering{index}Callee");
        uwriteln!(
            self.src.js,
            "const {callee} = imports.{}.{};",
            import_name.to_lower_camel_case(),
            func.name.to_lower_camel_case(),
        );
        uwrite!(self.src.js, "function lowering{index}");
        let nparams = iface
            .wasm_signature(AbiVariant::GuestImport, func)
            .params
            .len();
        self.bindgen(
            nparams,
            callee,
            &import.options,
            iface,
            func,
            AbiVariant::GuestImport,
        );
        uwriteln!(self.src.js, "");
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
        self.src.js(") {\n");

        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        let mut f = FunctionBindgen {
            sizes,
            gen: self.gen,
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
        format!("instance{i}.exports[\"{name}\"]")
    }

    fn exports(
        &mut self,
        exports: &IndexMap<String, Export>,
        depth: usize,
        iface: Option<&Interface>,
    ) {
        if exports.is_empty() {
            self.src.js("{}");
            return;
        }

        self.src.js("{\n");
        for (name, export) in exports {
            let camel = name.to_lower_camel_case();
            match export {
                Export::LiftedFunction {
                    ty: _,
                    func,
                    options,
                } => {
                    assert!(depth < 2);
                    uwrite!(self.src.js, "{camel}");
                    let callee = self.core_def(func);
                    let iface = iface.unwrap();
                    let func = iface.functions.iter().find(|f| f.name == *name).unwrap();
                    self.bindgen(
                        func.params.len(),
                        callee,
                        options,
                        iface,
                        func,
                        AbiVariant::GuestExport,
                    );
                    self.src.js(",\n");
                }
                Export::Instance(exports) => {
                    uwrite!(self.src.js, "{camel}: ");
                    let iface = self.interfaces.exports.get(name.as_str());
                    self.exports(exports, depth + 1, iface);
                    self.src.js(",\n");
                }

                // ignore type exports for now
                Export::Type(_) => {}

                // This can't be tested at this time so leave it unimplemented
                Export::Module(_) => unimplemented!(),
            }
        }
        self.src.js("}");
    }
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

    fn print_ty(&mut self, ty: &Type) {
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
                    TypeDefKind::Type(t) => self.print_ty(t),
                    TypeDefKind::Tuple(t) => self.print_tuple(t),
                    TypeDefKind::Record(_) => panic!("anonymous record"),
                    TypeDefKind::Flags(_) => panic!("anonymous flags"),
                    TypeDefKind::Enum(_) => panic!("anonymous enum"),
                    TypeDefKind::Union(_) => panic!("anonymous union"),
                    TypeDefKind::Option(t) => {
                        if self.maybe_null(t) {
                            self.needs_ty_option = true;
                            self.src.ts("Option<");
                            self.print_ty(t);
                            self.src.ts(">");
                        } else {
                            self.print_ty(t);
                            self.src.ts(" | null");
                        }
                    }
                    TypeDefKind::Result(r) => {
                        self.needs_ty_result = true;
                        self.src.ts("Result<");
                        self.print_optional_ty(r.ok.as_ref());
                        self.src.ts(", ");
                        self.print_optional_ty(r.err.as_ref());
                        self.src.ts(">");
                    }
                    TypeDefKind::Variant(_) => panic!("anonymous variant"),
                    TypeDefKind::List(v) => self.print_list(v),
                    TypeDefKind::Future(_) => todo!("anonymous future"),
                    TypeDefKind::Stream(_) => todo!("anonymous stream"),
                }
            }
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>) {
        match ty {
            Some(ty) => self.print_ty(ty),
            None => self.src.ts("void"),
        }
    }

    fn print_list(&mut self, ty: &Type) {
        match self.array_ty(ty) {
            Some(ty) => self.src.ts(ty),
            None => {
                self.print_ty(ty);
                self.src.ts("[]");
            }
        }
    }

    fn print_tuple(&mut self, tuple: &Tuple) {
        self.src.ts("[");
        for (i, ty) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.src.ts(", ");
            }
            self.print_ty(ty);
        }
        self.src.ts("]");
    }

    fn ts_func(&mut self, func: &Function) {
        self.docs(&func.docs);

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
            self.print_ty(ty);
        }
        self.src.ts("): ");
        match func.results.len() {
            0 => self.src.ts("void"),
            1 => self.print_ty(func.results.iter_types().next().unwrap()),
            _ => {
                self.src.ts("[");
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i != 0 {
                        self.src.ts(", ");
                    }
                    self.print_ty(ty);
                }
                self.src.ts("]");
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
                .ts("export type Option<T> = { tag: \"none\" } | { tag: \"some\", val; T };\n");
        }
        if mem::take(&mut self.needs_ty_result) {
            self.src.ts(
                "export type Result<T, E> = { tag: \"ok\", val: T } | { tag: \"err\", val: E };\n",
            );
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
            self.print_ty(ty);
            self.src.ts(",\n");
        }
        self.src.ts("}\n");
    }

    fn type_tuple(&mut self, _id: TypeId, name: &str, tuple: &Tuple, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_tuple(tuple);
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
            self.src.ts("tag: \"");
            self.src.ts(&case.name);
            self.src.ts("\",\n");
            if let Some(ty) = case.ty {
                self.src.ts("val: ");
                self.print_ty(&ty);
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
            self.print_ty(&case.ty);
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
            self.print_ty(payload);
            self.src.ts(">");
        } else {
            self.print_ty(payload);
            self.src.ts(" | null");
        }
        self.src.ts(";\n");
    }

    fn type_result(&mut self, _id: TypeId, name: &str, result: &Result_, docs: &Docs) {
        self.docs(docs);
        let name = name.to_upper_camel_case();
        self.needs_ty_result = true;
        self.src.ts(&format!("export type {name} = Result<"));
        self.print_optional_ty(result.ok.as_ref());
        self.src.ts(", ");
        self.print_optional_ty(result.err.as_ref());
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
            self.src.ts(&format!("\"{}\"", case.name));
        }
        self.src.ts(";\n");
    }

    fn type_alias(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_ty(ty);
        self.src.ts(";\n");
    }

    fn type_list(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_list(ty);
        self.src.ts(";\n");
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        drop((_id, name, ty, docs));
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut Js,
    sizes: SizeAlign,
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
                let throw = self.gen.intrinsic(Intrinsic::ThrowInvalidBool);
                results.push(format!(
                    "bool{tmp} == 0 ? false : (bool{tmp} == 1 ? true : {throw}())"
                ));
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
                    "if (typeof {op0} === \"object\" && {op0} !== null) {{\n"
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
                        throw new TypeError(\"only an object, undefined or null can be converted to flags\");
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
                    if flags.flags.len() % 32 != 0 {
                        let mask: u32 = 0xffffffff << (flags.flags.len() % 32);
                        self.src.js(&format!(
                            "\
                            if (({op} & {mask}) !== 0) {{
                                throw new TypeError('flags have extraneous bits set');
                            }}
                            "
                        ));
                    }
                }

                self.src.js(&format!("const flags{tmp} = {{\n"));

                for (i, flag) in flags.flags.iter().enumerate() {
                    let flag = flag.name.to_lower_camel_case();
                    let op = &operands[i / 32];
                    let mask: u32 = 1 << (i % 32);
                    self.src.js(&format!("{flag}: Boolean({op} & {mask}),\n"));
                }

                self.src.js("};\n");
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
                self.src
                    .js(&format!("const variant{} = {};\n", tmp, operands[0]));

                for i in 0..result_types.len() {
                    self.src.js(&format!("let variant{}_{};\n", tmp, i));
                    results.push(format!("variant{}_{}", tmp, i));
                }

                let expr_to_match = format!("variant{}.tag", tmp);

                self.src.js(&format!("switch ({}) {{\n", expr_to_match));
                for (case, (block, block_results)) in variant.cases.iter().zip(blocks) {
                    self.src
                        .js(&format!("case \"{}\": {{\n", case.name.as_str()));
                    if case.ty.is_some() {
                        self.src.js(&format!("const e = variant{}.val;\n", tmp));
                    }
                    self.src.js(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        self.src
                            .js(&format!("variant{}_{} = {};\n", tmp, i, result));
                    }
                    self.src.js("break;\n}\n");
                }
                let variant_name = name.to_upper_camel_case();
                self.src.js("default:\n");
                self.src.js(&format!(
                    "throw new RangeError(\"invalid variant specified for {}\");\n",
                    variant_name
                ));
                self.src.js("}\n");
            }

            Instruction::VariantLift { variant, name, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();

                let tmp = self.tmp();

                self.src.js(&format!("let variant{};\n", tmp));
                self.src.js(&format!("switch ({}) {{\n", operands[0]));
                for (i, (case, (block, block_results))) in
                    variant.cases.iter().zip(blocks).enumerate()
                {
                    self.src.js(&format!("case {}: {{\n", i));
                    self.src.js(&block);

                    self.src.js(&format!("variant{} = {{\n", tmp));
                    self.src.js(&format!("tag: \"{}\",\n", case.name.as_str()));
                    if case.ty.is_some() {
                        assert!(block_results.len() == 1);
                        self.src.js(&format!("val: {},\n", block_results[0]));
                    } else {
                        assert!(block_results.len() == 0);
                    }
                    self.src.js("};\n");
                    self.src.js("break;\n}\n");
                }
                let variant_name = name.to_upper_camel_case();
                self.src.js("default:\n");
                self.src.js(&format!(
                    "throw new RangeError(\"invalid variant discriminant for {}\");\n",
                    variant_name
                ));
                self.src.js("}\n");
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
                self.src.js(&format!("const union{tmp} = {op0};\n"));

                for i in 0..result_types.len() {
                    self.src.js(&format!("let union{tmp}_{i};\n"));
                    results.push(format!("union{tmp}_{i}"));
                }

                self.src.js(&format!("switch (union{tmp}.tag) {{\n"));
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    self.src.js(&format!("case {i}: {{\n"));
                    self.src.js(&format!("const e = union{tmp}.val;\n"));
                    self.src.js(&block);
                    for (i, result) in block_results.iter().enumerate() {
                        self.src.js(&format!("union{tmp}_{i} = {result};\n"));
                    }
                    self.src.js("break;\n}\n");
                }
                let name = name.to_upper_camel_case();
                self.src.js("default:\n");
                self.src.js(&format!(
                    "throw new RangeError(\"invalid union specified for {name}\");\n",
                ));
                self.src.js("}\n");
            }

            Instruction::UnionLift { union, name, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();

                let tmp = self.tmp();

                self.src.js(&format!("let union{tmp};\n"));
                self.src.js(&format!("switch ({}) {{\n", operands[0]));
                for (i, (_case, (block, block_results))) in
                    union.cases.iter().zip(blocks).enumerate()
                {
                    assert!(block_results.len() == 1);
                    let block_result = &block_results[0];
                    self.src.js(&format!(
                        "case {i}: {{
                            {block}
                            union{tmp} = {{
                                tag: {i},
                                val: {block_result},
                            }};
                            break;
                        }}\n"
                    ));
                }
                let name = name.to_upper_camel_case();
                self.src.js("default:\n");
                self.src.js(&format!(
                    "throw new RangeError(\"invalid union discriminant for {name}\");\n",
                ));
                self.src.js("}\n");
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
                self.src
                    .js(&format!("const variant{tmp} = {};\n", operands[0]));

                for i in 0..result_types.len() {
                    self.src.js(&format!("let variant{tmp}_{i};\n"));
                    results.push(format!("variant{tmp}_{i}"));

                    let some_result = &some_results[i];
                    let none_result = &none_results[i];
                    some.push_str(&format!("variant{tmp}_{i} = {some_result};\n"));
                    none.push_str(&format!("variant{tmp}_{i} = {none_result};\n"));
                }

                if self.gen.maybe_null(iface, payload) {
                    self.src.js(&format!(
                        "
                        switch (variant{tmp}.tag) {{
                            case \"none\": {{
                                {none}
                                break;
                            }}
                            case \"some\": {{
                                const e = variant{tmp}.val;
                                {some}
                                break;
                            }}
                            default: {{
                                throw new RangeError(\"invalid variant specified for option\");
                            }}
                        }}
                        "
                    ));
                } else {
                    self.src.js(&format!(
                        "
                        switch (variant{tmp}) {{
                            case null: {{
                                {none}
                                break;
                            }}
                            default: {{
                                const e = variant{tmp};
                                {some}
                                break;
                            }}
                        }}
                        "
                    ));
                }
            }

            Instruction::OptionLift { payload, .. } => {
                let (some, some_results) = self.blocks.pop().unwrap();
                let (none, none_results) = self.blocks.pop().unwrap();
                assert!(none_results.len() == 0);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];

                let tmp = self.tmp();

                self.src.js(&format!("let variant{tmp};\n"));
                self.src.js(&format!("switch ({}) {{\n", operands[0]));

                if self.gen.maybe_null(iface, payload) {
                    self.src.js(&format!(
                        "
                            case 0: {{
                                {none}
                                variant{tmp} = {{ tag: \"none\" }};
                                break;
                            }}
                            case 1: {{
                                {some}
                                variant{tmp} = {{ tag: \"some\", val: {some_result} }};
                                break;
                            }}
                        ",
                    ));
                } else {
                    self.src.js(&format!(
                        "
                            case 0: {{
                                {none}
                                variant{tmp} = null;
                                break;
                            }}
                            case 1: {{
                                {some}
                                variant{tmp} = {some_result};
                                break;
                            }}
                        ",
                    ));
                }
                self.src.js("
                    default:
                        throw new RangeError(\"invalid variant discriminant for option\");
                ");
                self.src.js("}\n");
                results.push(format!("variant{tmp}"));
            }

            Instruction::ResultLower {
                results: result_types,
                ..
            } => {
                let (mut err, err_results) = self.blocks.pop().unwrap();
                let (mut ok, ok_results) = self.blocks.pop().unwrap();

                let tmp = self.tmp();
                self.src
                    .js(&format!("const variant{tmp} = {};\n", operands[0]));

                for i in 0..result_types.len() {
                    self.src.js(&format!("let variant{tmp}_{i};\n"));
                    results.push(format!("variant{tmp}_{i}"));

                    let ok_result = &ok_results[i];
                    let err_result = &err_results[i];
                    ok.push_str(&format!("variant{tmp}_{i} = {ok_result};\n"));
                    err.push_str(&format!("variant{tmp}_{i} = {err_result};\n"));
                }

                self.src.js(&format!(
                    "
                    switch (variant{tmp}.tag) {{
                        case \"ok\": {{
                            const e = variant{tmp}.val;
                            {ok}
                            break;
                        }}
                        case \"err\": {{
                            const e = variant{tmp}.val;
                            {err}
                            break;
                        }}
                        default: {{
                            throw new RangeError(\"invalid variant specified for result\");
                        }}
                    }}
                    "
                ));
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
                self.src.js(&format!(
                    "
                    let variant{tmp};
                    switch ({op0}) {{
                        case 0: {{
                            {ok}
                            variant{tmp} = {{ tag: \"ok\", val: {ok_result} }};
                            break;
                        }}
                        case 1: {{
                            {err}
                            variant{tmp} = {{ tag: \"err\", val: {err_result} }};
                            break;
                        }}
                        default: {{
                            throw new RangeError(\"invalid variant discriminant for expected\");
                        }}
                    }}
                    ",
                ));
                results.push(format!("variant{tmp}"));
            }

            // Lowers an enum in accordance with https://webidl.spec.whatwg.org/#es-enumeration.
            Instruction::EnumLower { name, enum_, .. } => {
                let tmp = self.tmp();

                let to_string = self.gen.intrinsic(Intrinsic::ToString);
                self.src
                    .js(&format!("const val{tmp} = {to_string}({});\n", operands[0]));

                // Declare a variable to hold the result.
                self.src.js(&format!("let enum{tmp};\n"));

                self.src.js(&format!("switch (val{tmp}) {{\n"));
                for (i, case) in enum_.cases.iter().enumerate() {
                    self.src.js(&format!(
                        "\
                        case \"{case}\": {{
                            enum{tmp} = {i};
                            break;
                        }}
                        ",
                        case = case.name
                    ));
                }
                self.src.js(&format!("\
                        default: {{
                            throw new TypeError(`\"${{val{tmp}}}\" is not one of the cases of {name}`);
                        }}
                    }}
                "));

                results.push(format!("enum{tmp}"));
            }

            Instruction::EnumLift { name, enum_, .. } => {
                let tmp = self.tmp();

                self.src.js(&format!("let enum{tmp};\n"));

                self.src.js(&format!("switch ({}) {{\n", operands[0]));
                for (i, case) in enum_.cases.iter().enumerate() {
                    self.src.js(&format!(
                        "\
                        case {i}: {{
                            enum{tmp} = \"{case}\";
                            break;
                        }}
                        ",
                        case = case.name
                    ));
                }
                self.src.js(&format!(
                    "\
                        default: {{
                            throw new RangeError(\"invalid discriminant specified for {name}\");
                        }}
                    }}
                    ",
                    name = name.to_upper_camel_case()
                ));

                results.push(format!("enum{tmp}"));
            }

            Instruction::ListCanonLower { element, .. } => {
                let tmp = self.tmp();
                let memory = self.memory.as_ref().unwrap();
                let realloc = self.realloc.as_ref().unwrap();

                let size = self.sizes.size(element);
                let align = self.sizes.align(element);
                uwriteln!(self.src.js, "const val{tmp} = {};", operands[0]);
                uwriteln!(self.src.js, "const len{tmp} = val{tmp}.length;");
                uwriteln!(
                    self.src.js,
                    "const ptr{tmp} = {realloc}(0, 0, {align}, len{tmp} * {size});"
                );
                // TODO: this is the wrong endianness
                uwriteln!(
                    self.src.js,
                    "const src{tmp} = new Uint8Array(val{tmp}.buffer, val{tmp}.byteOffset, len{tmp} * {size});",
                );
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
                    self.gen.intrinsic(Intrinsic::IsLE);
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
                uwriteln!(self.src.js, "const base = {result} + i * {size};");
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

            Instruction::CallInterface { module: _, func } => {
                self.bind_results(func.results.len(), results);
                uwriteln!(self.src.js, "{}({});", self.callee, operands.join(", "));
            }

            Instruction::Return { amt, .. } => {
                if let Some(f) = &self.post_return {
                    uwriteln!(self.src.js, "{f}(ret);");
                }

                match amt {
                    0 => {}
                    1 => uwriteln!(self.src.js, "return {};", operands[0]),
                    _ => uwriteln!(self.src.js, "return [{}];", operands.join(", ")),
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

#[derive(Default)]
struct Source {
    js: wit_bindgen_core::Source,
    ts: wit_bindgen_core::Source,
}

impl Source {
    fn js(&mut self, s: &str) {
        self.js.push_str(s);
    }
    fn ts(&mut self, s: &str) {
        self.ts.push_str(s);
    }
}
