use heck::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::mem;
use wit_bindgen_gen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_gen_core::{wit_parser::*, Direction, Files, Generator};

#[derive(Default)]
pub struct Js {
    src: Source,
    in_import: bool,
    opts: Opts,
    guest_imports: HashMap<String, Imports>,
    guest_exports: HashMap<String, Exports>,
    sizes: SizeAlign,
    intrinsics: BTreeMap<Intrinsic, String>,
    all_intrinsics: BTreeSet<Intrinsic>,
    needs_get_export: bool,
    imported_resources: BTreeSet<ResourceId>,
    exported_resources: BTreeSet<ResourceId>,
    needs_ty_option: bool,
    needs_ty_result: bool,
}

#[derive(Default)]
struct Imports {
    freestanding_funcs: Vec<(String, Source)>,
    resource_funcs: BTreeMap<ResourceId, Vec<(String, Source)>>,
}

#[derive(Default)]
struct Exports {
    freestanding_funcs: Vec<Source>,
    resource_funcs: BTreeMap<ResourceId, Vec<Source>>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    #[cfg_attr(feature = "structopt", structopt(long = "no-typescript"))]
    pub no_typescript: bool,
}

impl Opts {
    pub fn build(self) -> Js {
        let mut r = Js::new();
        r.opts = self;
        r
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum Intrinsic {
    ClampGuest,
    ClampHost,
    ClampHost64,
    DataView,
    ValidateGuestChar,
    ValidateHostChar,
    ValidateFlags,
    ValidateFlags64,
    /// Implementation of https://tc39.es/ecma262/#sec-tostring.
    ToString,
    I32ToF32,
    F32ToI32,
    I64ToF64,
    F64ToI64,
    Utf8Decoder,
    Utf8Encode,
    Utf8EncodedLen,
    Slab,
    Promises,
    WithCurrentPromise,
    ThrowInvalidBool,
}

impl Intrinsic {
    fn name(&self) -> &'static str {
        match self {
            Intrinsic::ClampGuest => "clamp_guest",
            Intrinsic::ClampHost => "clamp_host",
            Intrinsic::ClampHost64 => "clamp_host64",
            Intrinsic::DataView => "data_view",
            Intrinsic::ValidateGuestChar => "validate_guest_char",
            Intrinsic::ValidateHostChar => "validate_host_char",
            Intrinsic::ValidateFlags => "validate_flags",
            Intrinsic::ValidateFlags64 => "validate_flags64",
            Intrinsic::ToString => "to_string",
            Intrinsic::F32ToI32 => "f32ToI32",
            Intrinsic::I32ToF32 => "i32ToF32",
            Intrinsic::F64ToI64 => "f64ToI64",
            Intrinsic::I64ToF64 => "i64ToF64",
            Intrinsic::Utf8Decoder => "UTF8_DECODER",
            Intrinsic::Utf8Encode => "utf8_encode",
            Intrinsic::Utf8EncodedLen => "UTF8_ENCODED_LEN",
            Intrinsic::Slab => "Slab",
            Intrinsic::Promises => "PROMISES",
            Intrinsic::WithCurrentPromise => "with_current_promise",
            Intrinsic::ThrowInvalidBool => "throw_invalid_bool",
        }
    }
}

impl Js {
    pub fn new() -> Js {
        Js::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the JS host-side
        // bindings, we don't use any extra adapter layer between guest wasm
        // modules and the host. When the guest imports functions using the
        // `GuestImport` ABI, the host directly implements the `GuestImport`
        // ABI, even though the host is *exporting* functions. Similarly, when
        // the guest exports functions using the `GuestExport` ABI, the host
        // directly imports them with the `GuestExport` ABI, even though the
        // host is *importing* functions.
        match dir {
            Direction::Import => AbiVariant::GuestExport,
            Direction::Export => AbiVariant::GuestImport,
        }
    }

    fn array_ty(&self, iface: &Interface, ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Unit | Type::Bool => None,
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
            Type::Handle(_) => None,
            Type::String => None,
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Type(t) => self.array_ty(iface, t),
                _ => None,
            },
        }
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type) {
        match ty {
            Type::Unit => self.src.ts("void"),
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
            Type::Handle(id) => self.src.ts(&iface.resources[*id].name.to_camel_case()),
            Type::String => self.src.ts("string"),
            Type::Id(id) => {
                let ty = &iface.types[*id];
                if let Some(name) = &ty.name {
                    return self.src.ts(&name.to_camel_case());
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.print_ty(iface, t),
                    TypeDefKind::Tuple(t) => self.print_tuple(iface, t),
                    TypeDefKind::Record(_) => panic!("anonymous record"),
                    TypeDefKind::Flags(_) => panic!("anonymous flags"),
                    TypeDefKind::Enum(_) => panic!("anonymous enum"),
                    TypeDefKind::Union(_) => panic!("anonymous union"),
                    TypeDefKind::Option(t) => {
                        if self.maybe_null(iface, t) {
                            self.needs_ty_option = true;
                            self.src.ts("Option<");
                            self.print_ty(iface, t);
                            self.src.ts(">");
                        } else {
                            self.print_ty(iface, t);
                            self.src.ts(" | null");
                        }
                    }
                    TypeDefKind::Expected(e) => {
                        self.needs_ty_result = true;
                        self.src.ts("Result<");
                        self.print_ty(iface, &e.ok);
                        self.src.ts(", ");
                        self.print_ty(iface, &e.err);
                        self.src.ts(">");
                    }
                    TypeDefKind::Variant(_) => panic!("anonymous variant"),
                    TypeDefKind::List(v) => self.print_list(iface, v),
                    TypeDefKind::Future(_) => todo!("anonymous future"),
                    TypeDefKind::Stream(_) => todo!("anonymous stream"),
                }
            }
        }
    }

    fn print_list(&mut self, iface: &Interface, ty: &Type) {
        match self.array_ty(iface, ty) {
            Some(ty) => self.src.ts(ty),
            None => {
                self.print_ty(iface, ty);
                self.src.ts("[]");
            }
        }
    }

    fn print_tuple(&mut self, iface: &Interface, tuple: &Tuple) {
        self.src.ts("[");
        for (i, ty) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.src.ts(", ");
            }
            self.print_ty(iface, ty);
        }
        self.src.ts("]");
    }

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

    fn ts_func(&mut self, iface: &Interface, func: &Function) {
        self.docs(&func.docs);

        let mut name_printed = false;
        if let FunctionKind::Static { .. } = &func.kind {
            // static methods in imports are still wired up to an imported host
            // object, but static methods on exports are actually static
            // methods on the resource object.
            if self.in_import {
                name_printed = true;
                self.src.ts(&func.name.to_mixed_case());
            } else {
                self.src.ts("static ");
            }
        }
        if !name_printed {
            self.src.ts(&func.item_name().to_mixed_case());
        }
        self.src.ts("(");

        let param_start = match &func.kind {
            FunctionKind::Freestanding => 0,
            FunctionKind::Static { .. } if self.in_import => 0,
            FunctionKind::Static { .. } => {
                // the 0th argument for exported static methods will be the
                // instantiated interface
                self.src.ts(&iface.name.to_mixed_case());
                self.src.ts(": ");
                self.src.ts(&iface.name.to_camel_case());
                if func.params.len() > 0 {
                    self.src.ts(", ");
                }
                0
            }
            // skip the first parameter on methods which is `this`
            FunctionKind::Method { .. } => 1,
        };

        for (i, (name, ty)) in func.params[param_start..].iter().enumerate() {
            if i > 0 {
                self.src.ts(", ");
            }
            self.src.ts(to_js_ident(&name.to_mixed_case()));
            self.src.ts(": ");
            self.print_ty(iface, ty);
        }
        self.src.ts("): ");
        if func.is_async {
            self.src.ts("Promise<");
        }
        self.print_ty(iface, &func.result);
        if func.is_async {
            self.src.ts(">");
        }
        self.src.ts(";\n");
    }

    fn intrinsic(&mut self, i: Intrinsic) -> String {
        if let Some(name) = self.intrinsics.get(&i) {
            return name.clone();
        }
        // TODO: should select a name that automatically doesn't conflict with
        // anything else being generated.
        self.intrinsics.insert(i, i.name().to_string());
        return i.name().to_string();
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

impl Generator for Js {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.sizes.fill(iface);
        self.in_import = variant == AbiVariant::GuestImport;
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.docs(docs);
        self.src
            .ts(&format!("export interface {} {{\n", name.to_camel_case()));
        for field in record.fields.iter() {
            self.docs(&field.docs);
            let (option_str, ty) = self
                .as_nullable(iface, &field.ty)
                .map_or(("", &field.ty), |ty| ("?", ty));
            self.src
                .ts(&format!("{}{}: ", field.name.to_mixed_case(), option_str));
            self.print_ty(iface, ty);
            self.src.ts(",\n");
        }
        self.src.ts("}\n");
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_camel_case()));
        self.print_tuple(iface, tuple);
        self.src.ts(";\n");
    }

    fn type_flags(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        self.docs(docs);
        let repr = js_flags_repr(flags);
        let ty = repr.ty();
        let suffix = repr.suffix();
        self.src
            .ts(&format!("export type {} = {ty};\n", name.to_camel_case()));
        let name = name.to_shouty_snake_case();
        for (i, flag) in flags.flags.iter().enumerate() {
            let flag = flag.name.to_shouty_snake_case();
            self.src.js(&format!(
                "export const {name}_{flag} = {}{suffix};\n",
                1u128 << i,
            ));
            self.src.ts(&format!(
                "export const {name}_{flag} = {}{suffix};\n",
                1u128 << i,
            ));
        }
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_camel_case()));
        for (i, case) in variant.cases.iter().enumerate() {
            if i > 0 {
                self.src.ts(" | ");
            }
            self.src
                .ts(&format!("{}_{}", name, case.name).to_camel_case());
        }
        self.src.ts(";\n");
        for case in variant.cases.iter() {
            self.docs(&case.docs);
            self.src.ts(&format!(
                "export interface {} {{\n",
                format!("{}_{}", name, case.name).to_camel_case()
            ));
            self.src.ts("tag: \"");
            self.src.ts(&case.name);
            self.src.ts("\",\n");
            if case.ty != Type::Unit {
                self.src.ts("val: ");
                self.print_ty(iface, &case.ty);
                self.src.ts(",\n");
            }
            self.src.ts("}\n");
        }
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        self.docs(docs);
        let name = name.to_camel_case();
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
            self.print_ty(iface, &case.ty);
            self.src.ts(",\n");
            self.src.ts("}\n");
        }
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        self.docs(docs);
        let name = name.to_camel_case();
        self.src.ts(&format!("export type {name} = "));
        if self.maybe_null(iface, payload) {
            self.needs_ty_option = true;
            self.src.ts("Option<");
            self.print_ty(iface, payload);
            self.src.ts(">");
        } else {
            self.print_ty(iface, payload);
            self.src.ts(" | null");
        }
        self.src.ts(";\n");
    }

    fn type_expected(
        &mut self,
        iface: &Interface,
        _id: TypeId,
        name: &str,
        expected: &Expected,
        docs: &Docs,
    ) {
        self.docs(docs);
        let name = name.to_camel_case();
        self.needs_ty_result = true;
        self.src.ts(&format!("export type {name} = Result<"));
        self.print_ty(iface, &expected.ok);
        self.src.ts(", ");
        self.print_ty(iface, &expected.err);
        self.src.ts(">;\n");
    }

    fn type_enum(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        enum_: &Enum,
        docs: &Docs,
    ) {
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
            .ts(&format!("export type {} = ", name.to_camel_case()));
        for (i, case) in enum_.cases.iter().enumerate() {
            if i != 0 {
                self.src.ts(" | ");
            }
            self.src.ts(&format!("\"{}\"", case.name));
        }
        self.src.ts(";\n");
    }

    fn type_resource(&mut self, _iface: &Interface, ty: ResourceId) {
        if !self.in_import {
            self.exported_resources.insert(ty);
        }
    }

    fn type_alias(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_camel_case()));
        self.print_ty(iface, ty);
        self.src.ts(";\n");
    }

    fn type_list(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.docs(docs);
        self.src
            .ts(&format!("export type {} = ", name.to_camel_case()));
        self.print_list(iface, ty);
        self.src.ts(";\n");
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        drop((iface, _id, name, ty, docs));
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);

        let sig = iface.wasm_signature(AbiVariant::GuestImport, func);
        let params = (0..sig.params.len())
            .map(|i| format!("arg{}", i))
            .collect::<Vec<_>>();
        self.src
            .js(&format!("function({}) {{\n", params.join(", ")));
        self.ts_func(iface, func);

        let mut f = FunctionBindgen::new(self, false, params);
        iface.call(
            AbiVariant::GuestImport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            ..
        } = f;

        if needs_memory {
            self.needs_get_export = true;
            // TODO: hardcoding "memory"
            self.src.js("const memory = get_export(\"memory\");\n");
        }

        if let Some(name) = needs_realloc {
            self.needs_get_export = true;
            self.src
                .js(&format!("const realloc = get_export(\"{}\");\n", name));
        }

        if let Some(name) = needs_free {
            self.needs_get_export = true;
            self.src
                .js(&format!("const free = get_export(\"{}\");\n", name));
        }
        self.src.js(&src.js);

        if func.is_async {
            // Note that `catch_closure` here is defined by the `CallInterface`
            // instruction.
            self.src.js("}, catch_closure);\n"); // `.then` block
            self.src.js("});\n"); // `with_current_promise` block.
        }
        self.src.js("}");

        let src = mem::replace(&mut self.src, prev);
        let imports = self
            .guest_imports
            .entry(iface.name.to_string())
            .or_insert(Imports::default());
        let dst = match &func.kind {
            FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                &mut imports.freestanding_funcs
            }
            FunctionKind::Method { resource, .. } => imports
                .resource_funcs
                .entry(*resource)
                .or_insert(Vec::new()),
        };
        dst.push((func.name.to_string(), src));
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "import" uses the "export" ABI variant on the inside of
    // this `Generator` implementation.
    fn import(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);

        let mut params = func
            .params
            .iter()
            .enumerate()
            .map(|(i, _)| format!("arg{}", i))
            .collect::<Vec<_>>();
        let mut sig_start = 0;
        let mut first_is_operand = true;
        let src_object = match &func.kind {
            FunctionKind::Freestanding => "this".to_string(),
            FunctionKind::Static { .. } => {
                self.src.js("static ");
                params.insert(0, iface.name.to_mixed_case());
                first_is_operand = false;
                iface.name.to_mixed_case()
            }
            FunctionKind::Method { .. } => {
                params[0] = "this".to_string();
                sig_start = 1;
                "this._obj".to_string()
            }
        };
        if func.is_async {
            self.src.js("async ");
        }
        self.src.js(&format!(
            "{}({}) {{\n",
            func.item_name().to_mixed_case(),
            params[sig_start..].join(", ")
        ));
        self.ts_func(iface, func);

        if !first_is_operand {
            params.remove(0);
        }
        let mut f = FunctionBindgen::new(self, false, params);
        f.src_object = src_object;
        iface.call(
            AbiVariant::GuestExport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );

        let FunctionBindgen {
            src,
            needs_memory,
            needs_realloc,
            needs_free,
            src_object,
            ..
        } = f;
        if needs_memory {
            // TODO: hardcoding "memory"
            self.src
                .js(&format!("const memory = {}._exports.memory;\n", src_object));
        }

        if let Some(name) = needs_realloc {
            self.src.js(&format!(
                "const realloc = {}._exports[\"{}\"];\n",
                src_object, name
            ));
        }

        if let Some(name) = needs_free {
            self.src.js(&format!(
                "const free = {}._exports[\"{}\"];\n",
                src_object, name
            ));
        }
        self.src.js(&src.js);
        self.src.js("}\n");

        let exports = self
            .guest_exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);

        let func_body = mem::replace(&mut self.src, prev);
        match &func.kind {
            FunctionKind::Freestanding => {
                exports.freestanding_funcs.push(func_body);
            }
            FunctionKind::Static { resource, .. } | FunctionKind::Method { resource, .. } => {
                exports
                    .resource_funcs
                    .entry(*resource)
                    .or_insert(Vec::new())
                    .push(func_body);
            }
        }
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        for (module, funcs) in mem::take(&mut self.guest_imports) {
            // TODO: `module.exports` vs `export function`
            self.src.js(&format!(
                "export function add{}ToImports(imports, obj{}) {{\n",
                module.to_camel_case(),
                if self.needs_get_export {
                    ", get_export"
                } else {
                    ""
                },
            ));
            self.src.ts(&format!(
                "export function add{}ToImports(imports: any, obj: {0}{}): void;\n",
                module.to_camel_case(),
                if self.needs_get_export {
                    ", get_export: (name: string) => WebAssembly.ExportValue"
                } else {
                    ""
                },
            ));
            self.src.js(&format!(
                "if (!(\"{0}\" in imports)) imports[\"{0}\"] = {{}};\n",
                module,
            ));

            self.src
                .ts(&format!("export interface {} {{\n", module.to_camel_case()));

            for (name, src) in funcs
                .freestanding_funcs
                .iter()
                .chain(funcs.resource_funcs.values().flat_map(|v| v))
            {
                self.src.js(&format!(
                    "imports[\"{}\"][\"{}\"] = {};\n",
                    module,
                    name,
                    src.js.trim(),
                ));
            }

            for (_, src) in funcs.freestanding_funcs.iter() {
                self.src.ts(&src.ts);
            }

            if self.imported_resources.len() > 0 {
                self.src
                    .js("if (!(\"canonical_abi\" in imports)) imports[\"canonical_abi\"] = {};\n");
            }
            for resource in self.imported_resources.clone() {
                let slab = self.intrinsic(Intrinsic::Slab);
                self.src.js(&format!(
                    "
                        const resources{idx} = new {slab}();
                        imports.canonical_abi[\"resource_drop_{name}\"] = (i) => {{
                            const val = resources{idx}.remove(i);
                            if (obj.drop{camel})
                                obj.drop{camel}(val);
                        }};
                    ",
                    name = iface.resources[resource].name,
                    camel = iface.resources[resource].name.to_camel_case(),
                    idx = resource.index(),
                    slab = slab,
                ));
                self.src.ts(&format!(
                    "drop{}?: (val: {0}) => void;\n",
                    iface.resources[resource].name.to_camel_case()
                ));
            }
            self.src.js("}");
            self.src.ts("}\n");

            for (resource, _) in iface.resources.iter() {
                self.src.ts(&format!(
                    "export interface {} {{\n",
                    iface.resources[resource].name.to_camel_case()
                ));
                if let Some(funcs) = funcs.resource_funcs.get(&resource) {
                    for (_, src) in funcs {
                        self.src.ts(&src.ts);
                    }
                }
                self.src.ts("}\n");
            }
        }
        let imports = mem::take(&mut self.src);

        for (module, exports) in mem::take(&mut self.guest_exports) {
            let module = module.to_camel_case();
            self.src.ts(&format!("export class {} {{\n", module));
            self.src.js(&format!("export class {} {{\n", module));

            self.src.ts("
               /**
                * The WebAssembly instance that this class is operating with.
                * This is only available after the `instantiate` method has
                * been called.
                */
                instance: WebAssembly.Instance;
            ");

            self.src.ts("
               /**
                * Constructs a new instance with internal state necessary to
                * manage a wasm instance.
                *
                * Note that this does not actually instantiate the WebAssembly
                * instance or module, you'll need to call the `instantiate`
                * method below to \"activate\" this class.
                */
                constructor();
            ");
            if self.exported_resources.len() > 0 {
                self.src.js("constructor() {\n");
                let slab = self.intrinsic(Intrinsic::Slab);
                for r in self.exported_resources.iter() {
                    self.src.js(&format!(
                        "this._resource{}_slab = new {}();\n",
                        r.index(),
                        slab
                    ));
                }
                self.src.js("}\n");
            }

            self.src.ts("
               /**
                * This is a low-level method which can be used to add any
                * intrinsics necessary for this instance to operate to an
                * import object.
                *
                * The `import` object given here is expected to be used later
                * to actually instantiate the module this class corresponds to.
                * If the `instantiate` method below actually does the
                * instantiation then there's no need to call this method, but
                * if you're instantiating manually elsewhere then this can be
                * used to prepare the import object for external instantiation.
                */
                addToImports(imports: any): void;
            ");
            self.src.js("addToImports(imports) {\n");
            let any_async = iface.functions.iter().any(|f| f.is_async);
            if self.exported_resources.len() > 0 || any_async {
                self.src
                    .js("if (!(\"canonical_abi\" in imports)) imports[\"canonical_abi\"] = {};\n");
            }
            for r in self.exported_resources.iter() {
                self.src.js(&format!(
                    "
                        imports.canonical_abi['resource_drop_{name}'] = i => {{
                            this._resource{idx}_slab.remove(i).drop();
                        }};
                        imports.canonical_abi['resource_clone_{name}'] = i => {{
                            const obj = this._resource{idx}_slab.get(i);
                            return this._resource{idx}_slab.insert(obj.clone())
                        }};
                        imports.canonical_abi['resource_get_{name}'] = i => {{
                            return this._resource{idx}_slab.get(i)._wasm_val;
                        }};
                        imports.canonical_abi['resource_new_{name}'] = i => {{
                            const registry = this._registry{idx};
                            return this._resource{idx}_slab.insert(new {class}(i, this));
                        }};
                    ",
                    name = iface.resources[*r].name,
                    idx = r.index(),
                    class = iface.resources[*r].name.to_camel_case(),
                ));
            }
            if any_async {
                let promises = self.intrinsic(Intrinsic::Promises);
                self.src.js(&format!(
                    "
                        imports.canonical_abi['async_export_done'] = (ctx, ptr) => {{
                            {}.remove(ctx)(ptr >>> 0)
                        }};
                    ",
                    promises
                ));
            }
            self.src.js("}\n");

            self.src.ts(&format!(
                "
                   /**
                    * Initializes this object with the provided WebAssembly
                    * module/instance.
                    *
                    * This is intended to be a flexible method of instantiating
                    * and completion of the initialization of this class. This
                    * method must be called before interacting with the
                    * WebAssembly object.
                    *
                    * The first argument to this method is where to get the
                    * wasm from. This can be a whole bunch of different types,
                    * for example:
                    *
                    * * A precompiled `WebAssembly.Module`
                    * * A typed array buffer containing the wasm bytecode.
                    * * A `Promise` of a `Response` which is used with
                    *   `instantiateStreaming`
                    * * A `Response` itself used with `instantiateStreaming`.
                    * * An already instantiated `WebAssembly.Instance`
                    *
                    * If necessary the module is compiled, and if necessary the
                    * module is instantiated. Whether or not it's necessary
                    * depends on the type of argument provided to
                    * instantiation.
                    *
                    * If instantiation is performed then the `imports` object
                    * passed here is the list of imports used to instantiate
                    * the instance. This method may add its own intrinsics to
                    * this `imports` object too.
                    */
                    instantiate(
                        module: WebAssembly.Module | BufferSource | Promise<Response> | Response | WebAssembly.Instance,
                        imports?: any,
                    ): Promise<void>;
                ",
            ));
            self.src.js("
                async instantiate(module, imports) {
                    imports = imports || {};
                    this.addToImports(imports);
            ");

            // With intrinsics prep'd we can now instantiate the module. JS has
            // a ... variety of methods of instantiation, so we basically just
            // try to be flexible here.
            self.src.js("
                if (module instanceof WebAssembly.Instance) {
                    this.instance = module;
                } else if (module instanceof WebAssembly.Module) {
                    this.instance = await WebAssembly.instantiate(module, imports);
                } else if (module instanceof ArrayBuffer || module instanceof Uint8Array) {
                    const { instance } = await WebAssembly.instantiate(module, imports);
                    this.instance = instance;
                } else {
                    const { instance } = await WebAssembly.instantiateStreaming(module, imports);
                    this.instance = instance;
                }
                this._exports = this.instance.exports;
            ");

            // Exported resources all get a finalization registry, and we
            // created them after instantiation so we can pass the raw wasm
            // export as the destructor callback.
            for r in self.exported_resources.iter() {
                self.src.js(&format!(
                    "this._registry{} = new FinalizationRegistry(this._exports['canonical_abi_drop_{}']);\n",
                    r.index(),
                    iface.resources[*r].name,
                ));
            }
            self.src.js("}\n");

            for func in exports.freestanding_funcs.iter() {
                self.src.js(&func.js);
                self.src.ts(&func.ts);
            }
            self.src.ts("}\n");
            self.src.js("}\n");

            for &ty in self.exported_resources.iter() {
                self.src.js(&format!(
                    "
                        export class {} {{
                            constructor(wasm_val, obj) {{
                                this._wasm_val = wasm_val;
                                this._obj = obj;
                                this._refcnt = 1;
                                obj._registry{idx}.register(this, wasm_val, this);
                            }}

                            clone() {{
                                this._refcnt += 1;
                                return this;
                            }}

                            drop() {{
                                this._refcnt -= 1;
                                if (this._refcnt !== 0)
                                    return;
                                this._obj._registry{idx}.unregister(this);
                                const dtor = this._obj._exports['canonical_abi_drop_{}'];
                                const wasm_val = this._wasm_val;
                                delete this._obj;
                                delete this._refcnt;
                                delete this._wasm_val;
                                dtor(wasm_val);
                            }}
                    ",
                    iface.resources[ty].name.to_camel_case(),
                    iface.resources[ty].name,
                    idx = ty.index(),
                ));
                self.src.ts(&format!(
                    "
                        export class {} {{
                            // Creates a new strong reference count as a new
                            // object.  This is only required if you're also
                            // calling `drop` below and want to manually manage
                            // the reference count from JS.
                            //
                            // If you don't call `drop`, you don't need to call
                            // this and can simply use the object from JS.
                            clone(): {0};

                            // Explicitly indicate that this JS object will no
                            // longer be used. If the internal reference count
                            // reaches zero then this will deterministically
                            // destroy the underlying wasm object.
                            //
                            // This is not required to be called from JS. Wasm
                            // destructors will be automatically called for you
                            // if this is not called using the JS
                            // `FinalizationRegistry`.
                            //
                            // Calling this method does not guarantee that the
                            // underlying wasm object is deallocated. Something
                            // else (including wasm) may be holding onto a
                            // strong reference count.
                            drop(): void;
                    ",
                    iface.resources[ty].name.to_camel_case(),
                ));

                if let Some(funcs) = exports.resource_funcs.get(&ty) {
                    for func in funcs {
                        self.src.js(&func.js);
                        self.src.ts(&func.ts);
                    }
                }

                self.src.ts("}\n");
                self.src.js("}\n");
            }
        }

        let exports = mem::take(&mut self.src);

        if mem::take(&mut self.needs_ty_option) {
            self.src
                .ts("export type Option<T> = { tag: \"none\" } | { tag: \"some\", val; T };\n");
        }
        if mem::take(&mut self.needs_ty_result) {
            self.src.ts(
                "export type Result<T, E> = { tag: \"ok\", val: T } | { tag: \"err\", val: E };\n",
            );
        }

        if self.intrinsics.len() > 0 {
            self.src.js("import { ");
            for (i, (intrinsic, name)) in mem::take(&mut self.intrinsics).into_iter().enumerate() {
                if i > 0 {
                    self.src.js(", ");
                }
                self.src.js(intrinsic.name());
                if intrinsic.name() != name {
                    self.src.js(" as ");
                    self.src.js(&name);
                }
                self.all_intrinsics.insert(intrinsic);
            }
            self.src.js(" } from './intrinsics.js';\n");
        }

        self.src.js(&imports.js);
        self.src.ts(&imports.ts);
        self.src.js(&exports.js);
        self.src.ts(&exports.ts);

        let src = mem::take(&mut self.src);
        let name = iface.name.to_kebab_case();
        files.push(&format!("{}.js", name), src.js.as_bytes());
        if !self.opts.no_typescript {
            files.push(&format!("{}.d.ts", name), src.ts.as_bytes());
        }
    }

    fn finish_all(&mut self, files: &mut Files) {
        assert!(self.src.ts.is_empty());
        assert!(self.src.js.is_empty());
        self.print_intrinsics();
        assert!(self.src.ts.is_empty());
        files.push("intrinsics.js", self.src.js.as_bytes());
    }
}

struct FunctionBindgen<'a> {
    gen: &'a mut Js,
    tmp: usize,
    src: Source,
    block_storage: Vec<wit_bindgen_gen_core::Source>,
    blocks: Vec<(String, Vec<String>)>,
    in_import: bool,
    needs_memory: bool,
    needs_realloc: Option<String>,
    needs_free: Option<String>,
    params: Vec<String>,
    src_object: String,
}

impl FunctionBindgen<'_> {
    fn new(gen: &mut Js, in_import: bool, params: Vec<String>) -> FunctionBindgen<'_> {
        FunctionBindgen {
            gen,
            tmp: 0,
            src: Source::default(),
            block_storage: Vec::new(),
            blocks: Vec::new(),
            in_import,
            needs_memory: false,
            needs_realloc: None,
            needs_free: None,
            params,
            src_object: "this".to_string(),
        }
    }

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

    fn clamp_host<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        let clamp = self.gen.intrinsic(Intrinsic::ClampHost);
        results.push(format!("{}({}, {}, {})", clamp, operands[0], min, max));
    }

    fn clamp_host64<T>(&mut self, results: &mut Vec<String>, operands: &[String], min: T, max: T)
    where
        T: std::fmt::Display,
    {
        let clamp = self.gen.intrinsic(Intrinsic::ClampHost64);
        results.push(format!("{}({}, {}n, {}n)", clamp, operands[0], min, max));
    }

    fn load(&mut self, method: &str, offset: i32, operands: &[String], results: &mut Vec<String>) {
        self.needs_memory = true;
        let view = self.gen.intrinsic(Intrinsic::DataView);
        results.push(format!(
            "{}(memory).{}({} + {}, true)",
            view, method, operands[0], offset,
        ));
    }

    fn store(&mut self, method: &str, offset: i32, operands: &[String]) {
        self.needs_memory = true;
        let view = self.gen.intrinsic(Intrinsic::DataView);
        self.src.js(&format!(
            "{}(memory).{}({} + {}, {}, true);\n",
            view, method, operands[1], offset, operands[0]
        ));
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
        &self.gen.sizes
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
            Instruction::I32FromU8 => self.clamp_host(results, operands, u8::MIN, u8::MAX),
            Instruction::I32FromS8 => self.clamp_host(results, operands, i8::MIN, i8::MAX),
            Instruction::I32FromU16 => self.clamp_host(results, operands, u16::MIN, u16::MAX),
            Instruction::I32FromS16 => self.clamp_host(results, operands, i16::MIN, i16::MAX),
            Instruction::I32FromU32 => {
                self.clamp_host(results, operands, u32::MIN, u32::MAX);
            }
            Instruction::I32FromS32 => self.clamp_host(results, operands, i32::MIN, i32::MAX),
            Instruction::I64FromU64 => self.clamp_host64(results, operands, u64::MIN, u64::MAX),
            Instruction::I64FromS64 => self.clamp_host64(results, operands, i64::MIN, i64::MAX),

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

            Instruction::UnitLower => {}
            Instruction::UnitLift => {
                results.push("undefined".to_string());
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

            // These instructions are used with handles when we're implementing
            // imports. This means we interact with the `resources` slabs to
            // translate the wasm-provided index into a JS value.
            Instruction::I32FromOwnedHandle { ty } => {
                self.gen.imported_resources.insert(*ty);
                results.push(format!("resources{}.insert({})", ty.index(), operands[0]));
            }
            Instruction::HandleBorrowedFromI32 { ty } => {
                self.gen.imported_resources.insert(*ty);
                results.push(format!("resources{}.get({})", ty.index(), operands[0]));
            }

            // These instructions are used for handles to objects owned in wasm.
            // This means that they're interacting with a wrapper class defined
            // in JS.
            Instruction::I32FromBorrowedHandle { ty } => {
                let tmp = self.tmp();
                self.src
                    .js(&format!("const obj{} = {};\n", tmp, operands[0]));

                // If this is the `this` argument then it's implicitly already valid
                if operands[0] != "this" {
                    self.src.js(&format!(
                        "if (!(obj{} instanceof {})) ",
                        tmp,
                        iface.resources[*ty].name.to_camel_case()
                    ));
                    self.src.js(&format!(
                        "throw new TypeError('expected instance of {}');\n",
                        iface.resources[*ty].name.to_camel_case()
                    ));
                }
                results.push(format!(
                    "{}._resource{}_slab.insert(obj{}.clone())",
                    self.src_object,
                    ty.index(),
                    tmp,
                ));
            }
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}._resource{}_slab.remove({})",
                    self.src_object,
                    ty.index(),
                    operands[0],
                ));
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
                    expr.push_str(&field.name.to_mixed_case());
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
                    result.push_str(&format!("{}: {},\n", field.name.to_mixed_case(), op));
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

            Instruction::FlagsLower { flags, .. } => {
                let repr = js_flags_repr(flags);
                let validate = match repr {
                    JsFlagsRepr::Number => self.gen.intrinsic(Intrinsic::ValidateFlags),
                    JsFlagsRepr::Bigint => self.gen.intrinsic(Intrinsic::ValidateFlags64),
                };
                let op0 = &operands[0];
                let len = flags.flags.len();
                let n = repr.suffix();
                let tmp = self.tmp();
                let mask = (1u128 << len) - 1;
                self.src.js(&format!(
                    "const flags{tmp} = {validate}({op0}, {mask}{n});\n"
                ));
                match repr {
                    JsFlagsRepr::Number => {
                        results.push(format!("flags{}", tmp));
                    }
                    JsFlagsRepr::Bigint => {
                        for i in 0..flags.repr().count() {
                            let i = 32 * i;
                            results.push(format!("Number((flags{tmp} >> {i}n) & 0xffffffffn)",));
                        }
                    }
                }
            }

            Instruction::FlagsLift { flags, .. } => {
                let repr = js_flags_repr(flags);
                let n = repr.suffix();
                let tmp = self.tmp();
                let operand = match repr {
                    JsFlagsRepr::Number => operands[0].clone(),
                    JsFlagsRepr::Bigint => {
                        self.src.js(&format!("let flags{tmp} = 0n;\n"));
                        for (i, op) in operands.iter().enumerate() {
                            let i = 32 * i;
                            self.src
                                .js(&format!("flags{tmp} |= BigInt({op}) << {i}n;\n",));
                        }
                        format!("flags{tmp}")
                    }
                };
                let validate = match repr {
                    JsFlagsRepr::Number => self.gen.intrinsic(Intrinsic::ValidateFlags),
                    JsFlagsRepr::Bigint => self.gen.intrinsic(Intrinsic::ValidateFlags64),
                };
                let len = flags.flags.len();
                let mask = (1u128 << len) - 1;
                results.push(format!("{validate}({operand}, {mask}{n})"));
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
                    if case.ty != Type::Unit {
                        self.src.js(&format!("const e = variant{}.val;\n", tmp));
                    }
                    self.src.js(&block);

                    for (i, result) in block_results.iter().enumerate() {
                        self.src
                            .js(&format!("variant{}_{} = {};\n", tmp, i, result));
                    }
                    self.src.js("break;\n}\n");
                }
                let variant_name = name.to_camel_case();
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
                    assert!(block_results.len() == 1);
                    if case.ty != Type::Unit {
                        self.src.js(&format!("val: {},\n", block_results[0]));
                    } else {
                        assert_eq!(block_results[0], "undefined");
                    }
                    self.src.js("};\n");
                    self.src.js("break;\n}\n");
                }
                let variant_name = name.to_camel_case();
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
                let name = name.to_camel_case();
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
                let name = name.to_camel_case();
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
                assert!(none_results.len() == 1);
                assert!(some_results.len() == 1);
                let some_result = &some_results[0];
                assert_eq!(none_results[0], "undefined");

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

            Instruction::ExpectedLower {
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
                            throw new RangeError(\"invalid variant specified for expected\");
                        }}
                    }}
                    "
                ));
            }

            Instruction::ExpectedLift { .. } => {
                let (err, err_results) = self.blocks.pop().unwrap();
                let (ok, ok_results) = self.blocks.pop().unwrap();
                let err_result = &err_results[0];
                let ok_result = &ok_results[0];
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
                    name = name.to_camel_case()
                ));

                results.push(format!("enum{tmp}"));
            }

            Instruction::ListCanonLower { element, realloc } => {
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.gen.needs_get_export = true;
                self.needs_memory = true;
                self.needs_realloc = Some(realloc.to_string());
                let tmp = self.tmp();

                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                self.src
                    .js(&format!("const val{} = {};\n", tmp, operands[0]));
                self.src.js(&format!("const len{} = val{0}.length;\n", tmp));
                self.src.js(&format!(
                    "const ptr{} = realloc(0, 0, {}, len{0} * {});\n",
                    tmp, align, size,
                ));
                // TODO: this is the wrong endianness
                self.src.js(&format!(
                    "(new Uint8Array(memory.buffer, ptr{0}, len{0} * {1})).set(new Uint8Array(val{0}.buffer, val{0}.byteOffset, len{0} * {1}));\n",
                    tmp, size,
                ));
                results.push(format!("ptr{}", tmp));
                results.push(format!("len{}", tmp));
            }
            Instruction::ListCanonLift { element, free, .. } => {
                self.needs_memory = true;
                let tmp = self.tmp();
                self.src
                    .js(&format!("const ptr{} = {};\n", tmp, operands[0]));
                self.src
                    .js(&format!("const len{} = {};\n", tmp, operands[1]));
                // TODO: this is the wrong endianness
                let array_ty = self.gen.array_ty(iface, element).unwrap();
                let result = format!(
                    "new {}(memory.buffer.slice(ptr{}, ptr{1} + len{1} * {}))",
                    array_ty,
                    tmp,
                    self.gen.sizes.size(element),
                );
                let align = self.gen.sizes.align(element);
                match free {
                    Some(free) => {
                        self.needs_free = Some(free.to_string());
                        self.src.js(&format!("const list{} = {};\n", tmp, result));
                        self.src
                            .js(&format!("free(ptr{}, len{0}, {});\n", tmp, align));
                        results.push(format!("list{}", tmp));
                    }
                    None => results.push(result),
                }
            }
            Instruction::StringLower { realloc } => {
                // Lowering only happens when we're passing strings into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let realloc = realloc.unwrap();
                self.gen.needs_get_export = true;
                self.needs_memory = true;
                self.needs_realloc = Some(realloc.to_string());
                let tmp = self.tmp();

                let encode = self.gen.intrinsic(Intrinsic::Utf8Encode);
                self.src.js(&format!(
                    "const ptr{} = {}({}, realloc, memory);\n",
                    tmp, encode, operands[0],
                ));
                let encoded_len = self.gen.intrinsic(Intrinsic::Utf8EncodedLen);
                self.src
                    .js(&format!("const len{} = {};\n", tmp, encoded_len));
                results.push(format!("ptr{}", tmp));
                results.push(format!("len{}", tmp));
            }
            Instruction::StringLift { free } => {
                self.needs_memory = true;
                let tmp = self.tmp();
                self.src
                    .js(&format!("const ptr{} = {};\n", tmp, operands[0]));
                self.src
                    .js(&format!("const len{} = {};\n", tmp, operands[1]));
                let decoder = self.gen.intrinsic(Intrinsic::Utf8Decoder);
                let result = format!(
                    "{}.decode(new Uint8Array(memory.buffer, ptr{}, len{1}))",
                    decoder, tmp,
                );
                match free {
                    Some(free) => {
                        self.needs_free = Some(free.to_string());
                        self.src.js(&format!("const list{} = {};\n", tmp, result));
                        self.src.js(&format!("free(ptr{}, len{0}, 1);\n", tmp));
                        results.push(format!("list{}", tmp));
                    }
                    None => results.push(result),
                }
            }

            Instruction::ListLower { element, realloc } => {
                let realloc = realloc.unwrap();
                let (body, body_results) = self.blocks.pop().unwrap();
                assert!(body_results.is_empty());
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                self.needs_realloc = Some(realloc.to_string());
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.src.js(&format!("const {} = {};\n", vec, operands[0]));
                self.src.js(&format!("const {} = {}.length;\n", len, vec));

                // ... then realloc space for the result in the guest module
                self.src.js(&format!(
                    "const {} = realloc(0, 0, {}, {} * {});\n",
                    result, align, len, size,
                ));

                // ... then consume the vector and use the block to lower the
                // result.
                self.src
                    .js(&format!("for (let i = 0; i < {}.length; i++) {{\n", vec));
                self.src.js(&format!("const e = {}[i];\n", vec));
                self.src
                    .js(&format!("const base = {} + i * {};\n", result, size));
                self.src.js(&body);
                self.src.js("}\n");

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free, .. } => {
                let (body, body_results) = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{}", tmp);
                self.src.js(&format!("const {} = {};\n", len, operands[1]));
                let base = format!("base{}", tmp);
                self.src.js(&format!("const {} = {};\n", base, operands[0]));
                let result = format!("result{}", tmp);
                self.src.js(&format!("const {} = [];\n", result));
                results.push(result.clone());

                self.src
                    .js(&format!("for (let i = 0; i < {}; i++) {{\n", len));
                self.src
                    .js(&format!("const base = {} + i * {};\n", base, size));
                self.src.js(&body);
                assert_eq!(body_results.len(), 1);
                self.src
                    .js(&format!("{}.push({});\n", result, body_results[0]));
                self.src.js("}\n");

                if let Some(free) = free {
                    self.needs_free = Some(free.to_string());
                    self.src
                        .js(&format!("free({}, {} * {}, {});\n", base, len, size, align,));
                }
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm {
                iface: _,
                name,
                sig,
            } => {
                self.bind_results(sig.results.len(), results);
                self.src.js(&self.src_object);
                self.src.js("._exports['");
                self.src.js(&name);
                self.src.js("'](");
                self.src.js(&operands.join(", "));
                self.src.js(");\n");
            }

            Instruction::CallWasmAsyncExport {
                module: _,
                name,
                params: _,
                results: wasm_results,
            } => {
                self.bind_results(wasm_results.len(), results);
                let promises = self.gen.intrinsic(Intrinsic::Promises);
                self.src.js(&format!(
                    "\
                        await new Promise((resolve, reject) => {{
                            const promise_ctx = {promises}.insert(val => {{
                                if (typeof val !== 'number')
                                    return reject(val);
                                resolve(\
                    ",
                    promises = promises
                ));

                if wasm_results.len() > 0 {
                    self.src.js("[");
                    let operands = &["val".to_string()];
                    let mut results = Vec::new();
                    for (i, result) in wasm_results.iter().enumerate() {
                        if i > 0 {
                            self.src.js(", ");
                        }
                        let method = match result {
                            WasmType::I32 => "getInt32",
                            WasmType::I64 => "getBigInt64",
                            WasmType::F32 => "getFloat32",
                            WasmType::F64 => "getFloat64",
                        };
                        self.load(method, (i * 8) as i32, operands, &mut results);
                        self.src.js(&results.pop().unwrap());
                    }
                    self.src.js("]");
                }

                // Finish the blocks from above
                self.src.js(");\n"); // `resolve(...)`
                self.src.js("});\n"); // `promises.insert(...)`

                let with = self.gen.intrinsic(Intrinsic::WithCurrentPromise);
                self.src.js(&with);
                self.src.js("(promise_ctx, _prev => {\n");
                self.src.js(&self.src_object);
                self.src.js("._exports['");
                self.src.js(&name);
                self.src.js("'](");
                for op in operands {
                    self.src.js(op);
                    self.src.js(", ");
                }
                self.src.js("promise_ctx);\n");
                self.src.js("});\n"); // call to `with`
                self.src.js("});\n"); // `await new Promise(...)`
            }

            Instruction::CallInterface { module: _, func } => {
                let call = |me: &mut FunctionBindgen<'_>| match &func.kind {
                    FunctionKind::Freestanding | FunctionKind::Static { .. } => {
                        me.src.js(&format!(
                            "obj.{}({})",
                            func.name.to_mixed_case(),
                            operands.join(", "),
                        ));
                    }
                    FunctionKind::Method { name, .. } => {
                        me.src.js(&format!(
                            "{}.{}({})",
                            operands[0],
                            name.to_mixed_case(),
                            operands[1..].join(", "),
                        ));
                    }
                };
                let mut bind_results = |me: &mut FunctionBindgen<'_>| match &func.result {
                    Type::Unit => {
                        results.push("".to_string());
                    }
                    _ => {
                        me.src.js("const ret = ");
                        results.push("ret".to_string());
                    }
                };

                if func.is_async {
                    let with = self.gen.intrinsic(Intrinsic::WithCurrentPromise);
                    let promises = self.gen.intrinsic(Intrinsic::Promises);
                    self.src.js(&with);
                    self.src.js("(null, cur_promise => {\n");
                    self.src.js(&format!(
                        "const catch_closure = e => {}.remove(cur_promise)(e);\n",
                        promises
                    ));
                    call(self);
                    self.src.js(".then(e => {\n");
                    match &func.result {
                        Type::Unit => {
                            results.push("".to_string());
                        }
                        _ => {
                            bind_results(self);
                            self.src.js("e;\n");
                        }
                    }
                } else {
                    bind_results(self);
                    call(self);
                    self.src.js(";\n");
                }
            }

            Instruction::Return { amt, func: _ } => match amt {
                0 => {}
                1 => self.src.js(&format!("return {};\n", operands[0])),
                _ => {
                    assert!(self.in_import);
                    self.src.js(&format!("return [{}];\n", operands.join(", ")));
                }
            },

            Instruction::ReturnAsyncImport { .. } => {
                // When we reenter webassembly successfully that means that the
                // host's promise resolved without exception. Take the current
                // promise index saved as part of `CallInterface` and update the
                // `CUR_PROMISE` global with what's currently being executed.
                // This'll get reset once the wasm returns again.
                //
                // Note that the name `cur_promise` used here is introduced in
                // the `CallInterface` codegen above in the closure for
                // `with_current_promise` which we're using here.
                //
                // TODO: hardcoding `__indirect_function_table` and no help if
                // it's not actually defined.
                self.gen.needs_get_export = true;
                let with = self.gen.intrinsic(Intrinsic::WithCurrentPromise);
                self.src.js(&format!(
                    "\
                        {with}(cur_promise, _prev => {{
                            get_export(\"__indirect_function_table\").get({})({});
                        }});
                    ",
                    operands[0],
                    operands[1..].join(", "),
                    with = with,
                ));
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

            Instruction::Malloc {
                realloc,
                size,
                align,
            } => {
                self.needs_realloc = Some(realloc.to_string());
                let tmp = self.tmp();
                let ptr = format!("ptr{}", tmp);
                self.src.js(&format!(
                    "const {} = realloc(0, 0, {}, {});\n",
                    ptr, align, size
                ));
                results.push(ptr);
            }

            i => unimplemented!("{:?}", i),
        }
    }
}

impl Js {
    fn print_intrinsics(&mut self) {
        if self.all_intrinsics.contains(&Intrinsic::I32ToF32)
            || self.all_intrinsics.contains(&Intrinsic::F32ToI32)
        {
            self.src.js("
                const I32_TO_F32_I = new Int32Array(1);
                const I32_TO_F32_F = new Float32Array(I32_TO_F32_I.buffer);
            ");
        }
        if self.all_intrinsics.contains(&Intrinsic::I64ToF64)
            || self.all_intrinsics.contains(&Intrinsic::F64ToI64)
        {
            self.src.js("
                const I64_TO_F64_I = new BigInt64Array(1);
                const I64_TO_F64_F = new Float64Array(I64_TO_F64_I.buffer);
            ");
        }

        if self.all_intrinsics.contains(&Intrinsic::Promises) {
            self.all_intrinsics.insert(Intrinsic::Slab);
        }

        for i in mem::take(&mut self.all_intrinsics) {
            self.print_intrinsic(i);
        }
    }

    fn print_intrinsic(&mut self, i: Intrinsic) {
        match i {
            Intrinsic::ClampGuest => self.src.js("
                export function clamp_guest(i, min, max) {
                    if (i < min || i > max) \
                        throw new RangeError(`must be between ${min} and ${max}`);
                    return i;
                }
            "),
            Intrinsic::ClampHost => self.src.js("
                export function clamp_host(i, min, max) {
                    if (!Number.isInteger(i)) \
                        throw new TypeError(`must be an integer`);
                    if (i < min || i > max) \
                        throw new RangeError(`must be between ${min} and ${max}`);
                    return i;
                }
            "),

            Intrinsic::DataView => self.src.js("
                let DATA_VIEW = new DataView(new ArrayBuffer());

                export function data_view(mem) {
                    if (DATA_VIEW.buffer !== mem.buffer) \
                        DATA_VIEW = new DataView(mem.buffer);
                    return DATA_VIEW;
                }
            "),

            Intrinsic::ClampHost64 => self.src.js("
                export function clamp_host64(i, min, max) {
                    if (typeof i !== 'bigint') \
                        throw new TypeError(`must be a bigint`);
                    if (i < min || i > max) \
                        throw new RangeError(`must be between ${min} and ${max}`);
                    return i;
                }
            "),

            Intrinsic::ValidateGuestChar => self.src.js("
                export function validate_guest_char(i) {
                    if ((i > 0x10ffff) || (i >= 0xd800 && i <= 0xdfff)) \
                        throw new RangeError(`not a valid char`);
                    return String.fromCodePoint(i);
                }
            "),

            // TODO: this is incorrect. It at least allows strings of length > 0
            // but it probably doesn't do the right thing for unicode or invalid
            // utf16 strings either.
            Intrinsic::ValidateHostChar => self.src.js("
                export function validate_host_char(s) {
                    if (typeof s !== 'string') \
                        throw new TypeError(`must be a string`);
                    return s.codePointAt(0);
                }
            "),

            Intrinsic::ValidateFlags => self.src.js("
                export function validate_flags(flags, mask) {
                    if (!Number.isInteger(flags)) \
                        throw new TypeError('flags were not an integer');
                    if ((flags & ~mask) != 0)
                        throw new TypeError('flags have extraneous bits set');
                    return flags;
                }
            "),

            Intrinsic::ValidateFlags64 => self.src.js("
                export function validate_flags64(flags, mask) {
                    if (typeof flags !== 'bigint')
                        throw new TypeError('flags were not a bigint');
                    if ((flags & ~mask) != 0n)
                        throw new TypeError('flags have extraneous bits set');
                    return flags;
                }
            "),

            Intrinsic::ToString => self.src.js("
                export function to_string(val) {
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
                export function i32ToF32(i) {
                    I32_TO_F32_I[0] = i;
                    return I32_TO_F32_F[0];
                }
            "),
            Intrinsic::F32ToI32 => self.src.js("
                export function f32ToI32(f) {
                    I32_TO_F32_F[0] = f;
                    return I32_TO_F32_I[0];
                }
            "),
            Intrinsic::I64ToF64 => self.src.js("
                export function i64ToF64(i) {
                    I64_TO_F64_I[0] = i;
                    return I64_TO_F64_F[0];
                }
            "),
            Intrinsic::F64ToI64 => self.src.js("
                export function f64ToI64(f) {
                    I64_TO_F64_F[0] = f;
                    return I64_TO_F64_I[0];
                }
            "),

            Intrinsic::Utf8Decoder => self
                .src
                .js("export const UTF8_DECODER = new TextDecoder('utf-8');\n"),

            Intrinsic::Utf8EncodedLen => self.src.js("export let UTF8_ENCODED_LEN = 0;\n"),

            Intrinsic::Utf8Encode => self.src.js("
                const UTF8_ENCODER = new TextEncoder('utf-8');

                export function utf8_encode(s, realloc, memory) {
                    if (typeof s !== 'string') \
                        throw new TypeError('expected a string');

                    if (s.length === 0) {
                        UTF8_ENCODED_LEN = 0;
                        return 1;
                    }

                    let alloc_len = 0;
                    let ptr = 0;
                    let writtenTotal = 0;
                    while (s.length > 0) {
                        ptr = realloc(ptr, alloc_len, 1, alloc_len + s.length);
                        alloc_len += s.length;
                        const { read, written } = UTF8_ENCODER.encodeInto(
                            s,
                            new Uint8Array(memory.buffer, ptr + writtenTotal, alloc_len - writtenTotal),
                        );
                        writtenTotal += written;
                        s = s.slice(read);
                    }
                    if (alloc_len > writtenTotal)
                        ptr = realloc(ptr, alloc_len, 1, writtenTotal);
                    UTF8_ENCODED_LEN = writtenTotal;
                    return ptr;
                }
            "),

            Intrinsic::Slab => self.src.js("
                export class Slab {
                    constructor() {
                        this.list = [];
                        this.head = 0;
                    }

                    insert(val) {
                        if (this.head >= this.list.length) {
                            this.list.push({
                                next: this.list.length + 1,
                                val: undefined,
                            });
                        }
                        const ret = this.head;
                        const slot = this.list[ret];
                        this.head = slot.next;
                        slot.next = -1;
                        slot.val = val;
                        return ret;
                    }

                    get(idx) {
                        if (idx >= this.list.length)
                            throw new RangeError('handle index not valid');
                        const slot = this.list[idx];
                        if (slot.next === -1)
                            return slot.val;
                        throw new RangeError('handle index not valid');
                    }

                    remove(idx) {
                        const ret = this.get(idx); // validate the slot
                        const slot = this.list[idx];
                        slot.val = undefined;
                        slot.next = this.head;
                        this.head = idx;
                        return ret;
                    }
                }
            "),

            Intrinsic::Promises => self.src.js("export const PROMISES = new Slab();\n"),
            Intrinsic::WithCurrentPromise => self.src.js("
                let CUR_PROMISE = null;
                export function with_current_promise(val, closure) {
                    const prev = CUR_PROMISE;
                    CUR_PROMISE = val;
                    try {
                        closure(prev);
                    } finally {
                        CUR_PROMISE = prev;
                    }
                }
            "),
            Intrinsic::ThrowInvalidBool => self.src.js("
                export function throw_invalid_bool() {
                    throw new RangeError(\"invalid variant discriminant for bool\");
                }
            "),
        }
    }
}

pub fn to_js_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "import" => "import_",
        s => s,
    }
}

#[derive(Default)]
struct Source {
    js: wit_bindgen_gen_core::Source,
    ts: wit_bindgen_gen_core::Source,
}

impl Source {
    fn js(&mut self, s: &str) {
        self.js.push_str(s);
    }
    fn ts(&mut self, s: &str) {
        self.ts.push_str(s);
    }
}

enum JsFlagsRepr {
    Number,
    Bigint,
}

impl JsFlagsRepr {
    fn ty(&self) -> &'static str {
        match self {
            JsFlagsRepr::Number => "number",
            JsFlagsRepr::Bigint => "bigint",
        }
    }
    fn suffix(&self) -> &'static str {
        match self {
            JsFlagsRepr::Number => "",
            JsFlagsRepr::Bigint => "n",
        }
    }
}

fn js_flags_repr(f: &Flags) -> JsFlagsRepr {
    match f.repr() {
        FlagsRepr::U8 | FlagsRepr::U16 | FlagsRepr::U32(1) => JsFlagsRepr::Number,
        FlagsRepr::U32(_) => JsFlagsRepr::Bigint,
    }
}
