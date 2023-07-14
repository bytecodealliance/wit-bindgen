use heck::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_bindgen_core::wit_parser::abi::{AbiVariant, Bindgen, Instruction, LiftLower, WasmType};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Source, TypeInfo, Types,
    WorldGenerator,
};
use wit_bindgen_rust_lib::{
    int_repr, to_rust_ident, wasm_type, FnSig, Ownership, RustFlagsRepr, RustFunctionGenerator,
    RustGenerator, TypeMode,
};

#[derive(Default)]
struct RustWasm {
    types: Types,
    src: Source,
    opts: Opts,
    exports: Vec<Source>,
    import_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    export_modules: BTreeMap<Option<PackageName>, Vec<String>>,
    skip: HashSet<String>,
    interface_names: HashMap<InterfaceId, String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rustfmt: bool,

    /// If true, code generation should qualify any features that depend on
    /// `std` with `cfg(feature = "std")`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub std_feature: bool,

    /// If true, adds `#[macro_export]` to the `export_*!` macro generated to
    /// export it from the Rust crate.
    #[cfg_attr(feature = "clap", arg(long))]
    pub macro_export: bool,

    /// If true, code generation should pass borrowed string arguments as
    /// `&[u8]` instead of `&str`. Strings are still required to be valid
    /// UTF-8, but this avoids the need for Rust code to do its own UTF-8
    /// validation if it doesn't already have a `&str`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub raw_strings: bool,

    /// The prefix to use when calling functions from within the generated
    /// `export!` macro.
    ///
    /// This enables the generated `export!` macro to reference code from
    /// another mod/crate.
    #[cfg_attr(feature = "clap", arg(long))]
    pub macro_call_prefix: Option<String>,

    /// The name of the generated `export!` macro to use.
    ///
    /// If `None`, the name is derived from the name of the world in the
    /// format `export_{world_name}!`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub export_macro_name: Option<String>,

    /// Names of functions to skip generating bindings for.
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip: Vec<String>,

    /// Whether to generate owning or borrowing type definitions.
    ///
    /// Valid values include:
    /// - `owning`: Generated types will be composed entirely of owning fields,
    /// regardless of whether they are used as parameters to imports or not.
    /// - `borrowing`: Generated types used as parameters to imports will be
    /// "deeply borrowing", i.e. contain references rather than owned values
    /// when applicable.
    /// - `borrowing-duplicate-if-necessary`: As above, but generating distinct
    /// types for borrowing and owning, if necessary.
    #[cfg_attr(feature = "clap", arg(long))]
    pub ownership: Ownership,

    /// The optional path to the wit-bindgen runtime module to use.
    ///
    /// This defaults to `wit_bindgen::rt`.
    #[cfg_attr(feature = "clap", arg(long))]
    pub runtime_path: Option<String>,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = RustWasm::new();
        r.skip = self.skip.iter().cloned().collect();
        r.opts = self;
        Box::new(r)
    }
}

impl RustWasm {
    fn new() -> RustWasm {
        RustWasm::default()
    }

    fn interface<'a>(
        &'a mut self,
        wasm_import_module: Option<&'a str>,
        resolve: &'a Resolve,
        in_import: bool,
    ) -> InterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(resolve);

        InterfaceGenerator {
            current_interface: None,
            wasm_import_module,
            src: Source::default(),
            in_import,
            gen: self,
            sizes,
            resolve,
            return_pointer_area_size: 0,
            return_pointer_area_align: 0,
        }
    }

    fn emit_modules(&mut self, modules: &BTreeMap<Option<PackageName>, Vec<String>>) {
        let mut map = BTreeMap::new();
        for (pkg, modules) in modules {
            match pkg {
                Some(pkg) => {
                    let prev = map
                        .entry(&pkg.namespace)
                        .or_insert(BTreeMap::new())
                        .insert(&pkg.name, modules);
                    assert!(prev.is_none());
                }
                None => {
                    for module in modules {
                        uwriteln!(self.src, "{module}");
                    }
                }
            }
        }
        for (ns, pkgs) in map {
            uwriteln!(self.src, "pub mod {} {{", ns.to_snake_case());
            for (pkg, modules) in pkgs {
                uwriteln!(self.src, "pub mod {} {{", pkg.to_snake_case());
                for module in modules {
                    uwriteln!(self.src, "{module}");
                }
                uwriteln!(self.src, "}}");
            }
            uwriteln!(self.src, "}}");
        }
    }

    fn runtime_path(&self) -> &str {
        self.opts
            .runtime_path
            .as_deref()
            .unwrap_or("wit_bindgen::rt")
    }
}

impl WorldGenerator for RustWasm {
    fn preprocess(&mut self, resolve: &Resolve, _world: WorldId) {
        let version = env!("CARGO_PKG_VERSION");
        uwriteln!(
            self.src,
            "// Generated by `wit-bindgen` {version}. DO NOT EDIT!"
        );
        self.types.analyze(resolve);
    }

    fn import_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let wasm_import_module = resolve.name_world_key(name);
        let mut gen = self.interface(Some(&wasm_import_module), resolve, true);
        gen.current_interface = Some((id, name));
        gen.types(id);

        for (_, func) in resolve.interfaces[id].functions.iter() {
            gen.generate_guest_import(func);
        }

        gen.finish_append_submodule(name);
    }

    fn import_funcs(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(Some("$root"), resolve, true);

        for (_, func) in funcs {
            gen.generate_guest_import(func);
        }

        let src = gen.finish();
        self.src.push_str(&src);
    }

    fn export_interface(
        &mut self,
        resolve: &Resolve,
        name: &WorldKey,
        id: InterfaceId,
        _files: &mut Files,
    ) {
        let mut gen = self.interface(None, resolve, false);
        gen.current_interface = Some((id, name));
        gen.types(id);
        let trait_name = match name {
            WorldKey::Name(name) => name.to_upper_camel_case(),
            WorldKey::Interface(id) => resolve.interfaces[*id]
                .name
                .as_ref()
                .unwrap()
                .to_upper_camel_case(),
        };
        gen.generate_exports(
            &trait_name,
            Some(name),
            resolve.interfaces[id].functions.values(),
        );
        gen.finish_append_submodule(name);
    }

    fn export_funcs(
        &mut self,
        resolve: &Resolve,
        world: WorldId,
        funcs: &[(&str, &Function)],
        _files: &mut Files,
    ) {
        let trait_name = &resolve.worlds[world].name.to_upper_camel_case();
        let mut gen = self.interface(None, resolve, false);
        gen.generate_exports(&trait_name, None, funcs.iter().map(|f| f.1));
        let src = gen.finish();
        self.src.push_str(&src);
    }

    fn export_types(
        &mut self,
        resolve: &Resolve,
        _world: WorldId,
        types: &[(&str, TypeId)],
        _files: &mut Files,
    ) {
        let mut gen = self.interface(None, resolve, false);
        for (name, ty) in types {
            gen.define_type(name, *ty);
        }
        let src = gen.finish();
        self.src.push_str(&src);
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId, files: &mut Files) {
        let name = &resolve.worlds[world].name;
        if !self.exports.is_empty() {
            let macro_name = if let Some(name) = self.opts.export_macro_name.as_ref() {
                name.to_snake_case()
            } else {
                format!("export_{}", name.to_snake_case())
            };
            let macro_export = if self.opts.macro_export {
                "#[macro_export]"
            } else {
                ""
            };
            uwrite!(
                self.src,
                "
                    /// Declares the export of the component's world for the
                    /// given type.
                    {macro_export}
                    macro_rules! {macro_name}(($t:ident) => {{
                        const _: () = {{
                "
            );
            for src in self.exports.iter() {
                self.src.push_str(src);
            }
            uwrite!(
                self.src,
                "
                        }};

                        #[used]
                        #[doc(hidden)]
                        #[cfg(target_arch = \"wasm32\")]
                        static __FORCE_SECTION_REF: fn() = {prefix}__link_section;
                    }});
                ",
                prefix = self.opts.macro_call_prefix.as_deref().unwrap_or("")
            );
        }

        let imports = mem::take(&mut self.import_modules);
        self.emit_modules(&imports);
        let exports = mem::take(&mut self.export_modules);
        if !exports.is_empty() {
            self.src.push_str("pub mod exports {\n");
            self.emit_modules(&exports);
            self.src.push_str("}\n");
        }

        self.src.push_str("\n#[cfg(target_arch = \"wasm32\")]\n");

        // The custom section name here must start with "component-type" but
        // otherwise is attempted to be unique here to ensure that this doesn't get
        // concatenated to other custom sections by LLD by accident since LLD will
        // concatenate custom sections of the same name.
        self.src
            .push_str(&format!("#[link_section = \"component-type:{}\"]\n", name,));

        let mut producers = wasm_metadata::Producers::empty();
        producers.add(
            "processed-by",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        );

        let component_type = wit_component::metadata::encode(
            resolve,
            world,
            wit_component::StringEncoding::UTF8,
            Some(&producers),
        )
        .unwrap();

        self.src.push_str("#[doc(hidden)]\n");
        self.src.push_str(&format!(
            "pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; {}] = ",
            component_type.len()
        ));
        self.src.push_str(&format!("{:?};\n", component_type));

        self.src.push_str(
            "
            #[inline(never)]
            #[doc(hidden)]
            #[cfg(target_arch = \"wasm32\")]
            pub fn __link_section() {}
        ",
        );

        let mut src = mem::take(&mut self.src);
        if self.opts.rustfmt {
            let mut child = Command::new("rustfmt")
                .arg("--edition=2018")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn `rustfmt`");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(src.as_bytes())
                .unwrap();
            src.as_mut_string().truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(src.as_mut_string())
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }

        let module_name = name.to_snake_case();
        files.push(&format!("{module_name}.rs"), src.as_bytes());
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    current_interface: Option<(InterfaceId, &'a WorldKey)>,
    in_import: bool,
    sizes: SizeAlign,
    gen: &'a mut RustWasm,
    wasm_import_module: Option<&'a str>,
    resolve: &'a Resolve,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
}

impl InterfaceGenerator<'_> {
    fn generate_exports<'a>(
        &mut self,
        trait_name: &str,
        interface_name: Option<&WorldKey>,
        funcs: impl Iterator<Item = &'a Function> + Clone,
    ) {
        uwriteln!(self.src, "pub trait {trait_name} {{");
        for func in funcs.clone() {
            if self.gen.skip.contains(&func.name) {
                continue;
            }
            let mut sig = FnSig::default();
            sig.private = true;
            self.print_signature(func, TypeMode::Owned, &sig);
            self.src.push_str(";\n");
        }
        uwriteln!(self.src, "}}");

        for func in funcs {
            self.generate_guest_export(func, interface_name, &trait_name);
        }
    }

    fn finish(&mut self) -> String {
        if self.return_pointer_area_align > 0 {
            uwrite!(
                self.src,
                "
                    #[allow(unused_imports)]
                    use {rt}::{{alloc, vec::Vec, string::String}};

                    #[repr(align({align}))]
                    struct _RetArea([u8; {size}]);
                    static mut _RET_AREA: _RetArea = _RetArea([0; {size}]);
                ",
                rt = self.gen.runtime_path(),
                align = self.return_pointer_area_align,
                size = self.return_pointer_area_size,
            );
        }

        mem::take(&mut self.src).into()
    }

    fn finish_append_submodule(mut self, name: &WorldKey) {
        let module = self.finish();
        let snake = match name {
            WorldKey::Name(name) => to_rust_ident(name),
            WorldKey::Interface(id) => {
                to_rust_ident(self.resolve.interfaces[*id].name.as_ref().unwrap())
            }
        };
        let mut path_to_root = String::from("super::");
        let pkg = match name {
            WorldKey::Name(_) => None,
            WorldKey::Interface(id) => {
                let pkg = self.resolve.interfaces[*id].package.unwrap();
                Some(self.resolve.packages[pkg].name.clone())
            }
        };
        if let Some((id, _)) = self.current_interface {
            let mut path = String::new();
            if !self.in_import {
                path.push_str("exports::");
                path_to_root.push_str("super::");
            }
            if let Some(name) = &pkg {
                path.push_str(&format!(
                    "{}::{}::",
                    name.namespace.to_snake_case(),
                    name.name.to_snake_case()
                ));
                path_to_root.push_str("super::super::");
            }
            path.push_str(&snake);
            self.gen.interface_names.insert(id, path);
        }
        let module = format!(
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[used]
                    #[doc(hidden)]
                    #[cfg(target_arch = \"wasm32\")]
                    static __FORCE_SECTION_REF: fn() = {path_to_root}__link_section;

                    {module}
                }}
            ",
        );
        let map = if self.in_import {
            &mut self.gen.import_modules
        } else {
            &mut self.gen.export_modules
        };
        map.entry(pkg).or_insert(Vec::new()).push(module);
    }

    fn generate_guest_import(&mut self, func: &Function) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let sig = FnSig::default();
        let param_mode = TypeMode::AllBorrowed("'_");
        match &func.kind {
            FunctionKind::Constructor(_) | FunctionKind::Method(_) | FunctionKind::Static(_) => {
                todo!("implement resources")
            }
            FunctionKind::Freestanding => {}
        }
        self.src.push_str("#[allow(clippy::all)]\n");
        let params = self.print_signature(func, param_mode, &sig);
        self.src.push_str("{\n");
        self.src.push_str(&format!(
            "
                #[allow(unused_imports)]
                use {rt}::{{alloc, vec::Vec, string::String}};
            ",
            rt = self.gen.runtime_path()
        ));
        self.src.push_str("unsafe {\n");

        let mut f = FunctionBindgen::new(self, params);
        f.gen.resolve.call(
            AbiVariant::GuestImport,
            LiftLower::LowerArgsLiftResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_cleanup_list,
            src,
            import_return_pointer_area_size,
            import_return_pointer_area_align,
            ..
        } = f;

        if needs_cleanup_list {
            self.src.push_str("let mut cleanup_list = Vec::new();\n");
        }
        if import_return_pointer_area_size > 0 {
            uwrite!(
                self.src,
                "
                    #[repr(align({import_return_pointer_area_align}))]
                    struct RetArea([u8; {import_return_pointer_area_size}]);
                    let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
                ",
            );
        }
        self.src.push_str(&String::from(src));

        self.src.push_str("}\n");
        self.src.push_str("}\n");

        match &func.kind {
            FunctionKind::Constructor(_) | FunctionKind::Method(_) | FunctionKind::Static(_) => {
                todo!("implement resources")
            }
            FunctionKind::Freestanding => {}
        }
    }

    fn generate_guest_export(
        &mut self,
        func: &Function,
        interface_name: Option<&WorldKey>,
        trait_bound: &str,
    ) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let name_snake = func.name.to_snake_case();
        let wasm_module_export_name = interface_name.map(|k| self.resolve.name_world_key(k));
        let export_name = func.core_export_name(wasm_module_export_name.as_deref());
        let mut macro_src = Source::default();
        // Generate, simultaneously, the actual lifting/lowering function within
        // the original module (`call_{name_snake}`) as well as the function
        // which will ge exported from the wasm module itself through the export
        // macro, `export_...` here.
        //
        // Both have the same type signature, but the one in the module is
        // generic while the one in the macro uses `$t` as the name to delegate
        // to and substitute as the generic.
        uwrite!(
            self.src,
            "
                #[doc(hidden)]
                pub unsafe fn call_{name_snake}<T: {trait_bound}>(\
            ",
        );
        uwrite!(
            macro_src,
            "
            const _: () = {{
                #[doc(hidden)]
                #[export_name = \"{export_name}\"]
                #[allow(non_snake_case)]
                unsafe extern \"C\" fn __export_{name_snake}(\
            ",
        );

        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            uwrite!(self.src, "{name}: {},", wasm_type(*param));
            uwrite!(macro_src, "{name}: {},", wasm_type(*param));
            params.push(name);
        }
        self.src.push_str(")");
        macro_src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                uwrite!(self.src, " -> {}", wasm_type(sig.results[0]));
                uwrite!(macro_src, " -> {}", wasm_type(sig.results[0]));
            }
            _ => unimplemented!(),
        }

        self.push_str(" {\n");

        uwrite!(
            self.src,
            "
                #[allow(unused_imports)]
                use {rt}::{{alloc, vec::Vec, string::String}};

                // Before executing any other code, use this function to run all static
                // constructors, if they have not yet been run. This is a hack required
                // to work around wasi-libc ctors calling import functions to initialize
                // the environment.
                //
                // This functionality will be removed once rust 1.69.0 is stable, at which
                // point wasi-libc will no longer have this behavior.
                //
                // See
                // https://github.com/bytecodealliance/preview2-prototyping/issues/99
                // for more details.
                #[cfg(target_arch=\"wasm32\")]
                {rt}::run_ctors_once();

            ",
            rt = self.gen.runtime_path()
        );

        // Finish out the macro-generated export implementation.
        macro_src.push_str(" {\n");
        let mut prefix = self
            .gen
            .opts
            .macro_call_prefix
            .clone()
            .unwrap_or(String::new());
        match interface_name {
            Some(WorldKey::Name(name)) => {
                prefix.push_str(&format!("exports::{}::", name.to_snake_case()));
            }
            Some(WorldKey::Interface(id)) => {
                let iface = &self.resolve.interfaces[*id];
                let pkg = &self.resolve.packages[iface.package.unwrap()];
                prefix.push_str(&format!(
                    "exports::{}::{}::{}::",
                    pkg.name.namespace.to_snake_case(),
                    pkg.name.name.to_snake_case(),
                    iface.name.as_ref().unwrap().to_snake_case()
                ));
            }
            None => {}
        }

        uwrite!(macro_src, "{prefix}call_{name_snake}::<$t>(",);
        for param in params.iter() {
            uwrite!(macro_src, "{param},");
        }
        uwriteln!(macro_src, ")\n}}"); // close function call and function
        uwriteln!(macro_src, "\n}};"); // close `const _: () = { ...`

        let mut f = FunctionBindgen::new(self, params);
        f.gen.resolve.call(
            AbiVariant::GuestExport,
            LiftLower::LiftArgsLowerResults,
            func,
            &mut f,
        );
        let FunctionBindgen {
            needs_cleanup_list,
            src,
            ..
        } = f;
        assert!(!needs_cleanup_list);
        self.src.push_str(&String::from(src));
        self.src.push_str("}\n");

        if self.resolve.guest_export_needs_post_return(func) {
            // Like above, generate both a generic function in the module itself
            // as well as something to go in the export macro.
            uwrite!(
                self.src,
                "
                    #[doc(hidden)]
                    pub unsafe fn post_return_{name_snake}<T: {trait_bound}>(\
                "
            );
            uwrite!(
                macro_src,
                "
                    const _: () = {{
                    #[doc(hidden)]
                    #[export_name = \"cabi_post_{export_name}\"]
                    #[allow(non_snake_case)]
                    unsafe extern \"C\" fn __post_return_{name_snake}(\
                "
            );
            let mut params = Vec::new();
            for (i, result) in sig.results.iter().enumerate() {
                let name = format!("arg{}", i);
                uwrite!(self.src, "{name}: {},", wasm_type(*result));
                uwrite!(macro_src, "{name}: {},", wasm_type(*result));
                params.push(name);
            }
            self.src.push_str(") {\n");
            macro_src.push_str(") {\n");

            // Finish out the macro here
            uwrite!(macro_src, "{prefix}post_return_{name_snake}::<$t>(");
            for param in params.iter() {
                uwrite!(macro_src, "{param},");
            }
            uwriteln!(macro_src, ")\n}}"); // close function call and function
            uwriteln!(macro_src, "\n}};"); // close `const _: () = { ...`

            let mut f = FunctionBindgen::new(self, params);
            f.gen.resolve.post_return(func, &mut f);
            let FunctionBindgen {
                needs_cleanup_list,
                src,
                ..
            } = f;
            assert!(!needs_cleanup_list);
            self.src.push_str(&String::from(src));
            self.src.push_str("}\n");
        }

        self.gen.exports.push(macro_src);
    }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn ownership(&self) -> Ownership {
        self.gen.opts.ownership
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let mut path = String::new();
        if let Some((cur, name)) = self.current_interface {
            if cur == interface {
                return None;
            }
            if !self.in_import {
                path.push_str("super::");
            }
            match name {
                WorldKey::Name(_) => {
                    path.push_str("super::");
                }
                WorldKey::Interface(_) => {
                    path.push_str("super::super::super::");
                }
            }
        }
        let name = &self.gen.interface_names[&interface];
        path.push_str(&name);
        Some(path)
    }

    fn std_feature(&self) -> bool {
        self.gen.opts.std_feature
    }

    fn use_raw_strings(&self) -> bool {
        self.gen.opts.raw_strings
    }

    fn push_vec_name(&mut self) {
        self.push_str(&format!("{rt}::vec::Vec", rt = self.gen.runtime_path()));
    }

    fn push_string_name(&mut self) {
        self.push_str(&format!(
            "{rt}::string::String",
            rt = self.gen.runtime_path()
        ));
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.gen.types.get(ty)
    }

    fn types_mut(&mut self) -> &mut Types {
        &mut self.gen.types
    }

    fn print_borrowed_slice(
        &mut self,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
        mode: TypeMode,
    ) {
        self.print_rust_slice(mutbl, ty, lifetime, mode);
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if self.gen.opts.raw_strings {
            self.push_str("[u8]");
        } else {
            self.push_str("str");
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &Record, docs: &Docs) {
        self.print_typedef_record(id, record, docs, false);
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_typedef_tuple(id, tuple, docs);
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.src.push_str("wit_bindgen::bitflags::bitflags! {\n");
        self.rustdoc(docs);
        let repr = RustFlagsRepr::new(flags);
        self.src.push_str(&format!(
            "#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]\npub struct {}: {repr} {{\n",
            name.to_upper_camel_case(),
        ));
        for (i, flag) in flags.flags.iter().enumerate() {
            self.rustdoc(&flag.docs);
            self.src.push_str(&format!(
                "const {} = 1 << {};\n",
                flag.name.to_shouty_snake_case(),
                i,
            ));
        }
        self.src.push_str("}\n");
        self.src.push_str("}\n");
    }

    fn type_variant(&mut self, id: TypeId, _name: &str, variant: &Variant, docs: &Docs) {
        self.print_typedef_variant(id, variant, docs, false);
    }

    fn type_union(&mut self, id: TypeId, _name: &str, union: &Union, docs: &Docs) {
        self.print_typedef_union(id, union, docs, false);
    }

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        self.print_typedef_option(id, payload, docs);
    }

    fn type_result(&mut self, id: TypeId, _name: &str, result: &Result_, docs: &Docs) {
        self.print_typedef_result(id, result, docs);
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs, &[], Box::new(|_| String::new()));
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(id, ty, docs);
    }

    fn type_list(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_type_list(id, ty, docs);
    }

    fn type_builtin(&mut self, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_upper_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(ty, TypeMode::Owned);
        self.src.push_str(";\n");
    }
}

struct FunctionBindgen<'a, 'b> {
    gen: &'b mut InterfaceGenerator<'a>,
    params: Vec<String>,
    src: Source,
    blocks: Vec<String>,
    block_storage: Vec<(Source, Vec<(String, String)>)>,
    tmp: usize,
    needs_cleanup_list: bool,
    cleanup: Vec<(String, String)>,
    import_return_pointer_area_size: usize,
    import_return_pointer_area_align: usize,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(gen: &'b mut InterfaceGenerator<'a>, params: Vec<String>) -> FunctionBindgen<'a, 'b> {
        FunctionBindgen {
            gen,
            params,
            src: Default::default(),
            blocks: Vec::new(),
            block_storage: Vec::new(),
            tmp: 0,
            needs_cleanup_list: false,
            cleanup: Vec::new(),
            import_return_pointer_area_size: 0,
            import_return_pointer_area_align: 0,
        }
    }

    fn emit_cleanup(&mut self) {
        for (ptr, layout) in mem::take(&mut self.cleanup) {
            self.push_str(&format!(
                "if {layout}.size() != 0 {{\nalloc::dealloc({ptr}, {layout});\n}}\n"
            ));
        }
        if self.needs_cleanup_list {
            self.push_str(
                "for (ptr, layout) in cleanup_list {\n
                    if layout.size() != 0 {\n
                        alloc::dealloc(ptr, layout);\n
                    }\n
                }\n",
            );
        }
    }

    fn declare_import(
        &mut self,
        module_name: &str,
        name: &str,
        params: &[WasmType],
        results: &[WasmType],
    ) -> String {
        // Define the actual function we're calling inline
        uwriteln!(
            self.src,
            "
                #[link(wasm_import_module = \"{module_name}\")]
                extern \"C\" {{
                    #[cfg_attr(target_arch = \"wasm32\", link_name = \"{name}\")]
                    #[cfg_attr(not(target_arch = \"wasm32\"), link_name = \"{module_name}_{name}\")]
                    fn wit_import(\
            "
        );
        for param in params.iter() {
            self.push_str("_: ");
            self.push_str(wasm_type(*param));
            self.push_str(", ");
        }
        self.push_str(")");
        assert!(results.len() < 2);
        for result in results.iter() {
            self.push_str(" -> ");
            self.push_str(wasm_type(*result));
        }
        self.push_str(";\n}\n");
        "wit_import".to_string()
    }
}

impl RustFunctionGenerator for FunctionBindgen<'_, '_> {
    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn rust_gen(&self) -> &dyn RustGenerator {
        self.gen
    }

    fn lift_lower(&self) -> LiftLower {
        if self.gen.in_import {
            LiftLower::LowerArgsLiftResults
        } else {
            LiftLower::LiftArgsLowerResults
        }
    }
}

impl Bindgen for FunctionBindgen<'_, '_> {
    type Operand = String;

    fn push_block(&mut self) {
        let prev_src = mem::take(&mut self.src);
        let prev_cleanup = mem::take(&mut self.cleanup);
        self.block_storage.push((prev_src, prev_cleanup));
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        if self.cleanup.len() > 0 {
            self.needs_cleanup_list = true;
            self.push_str("cleanup_list.extend_from_slice(&[");
            for (ptr, layout) in mem::take(&mut self.cleanup) {
                self.push_str("(");
                self.push_str(&ptr);
                self.push_str(", ");
                self.push_str(&layout);
                self.push_str("),");
            }
            self.push_str("]);\n");
        }
        let (prev_src, prev_cleanup) = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, prev_src);
        self.cleanup = prev_cleanup;
        let expr = match operands.len() {
            0 => "()".to_string(),
            1 => operands[0].clone(),
            _ => format!("({})", operands.join(", ")),
        };
        if src.is_empty() {
            self.blocks.push(expr);
        } else if operands.is_empty() {
            self.blocks.push(format!("{{\n{}\n}}", &src[..]));
        } else {
            self.blocks.push(format!("{{\n{}\n{}\n}}", &src[..], expr));
        }
    }

    fn return_pointer(&mut self, size: usize, align: usize) -> String {
        let tmp = self.tmp();

        // Imports get a per-function return area to facilitate using the
        // stack whereas exports use a per-module return area to cut down on
        // stack usage. Note that for imports this also facilitates "adapter
        // modules" for components to not have data segments.
        if self.gen.in_import {
            self.import_return_pointer_area_size = self.import_return_pointer_area_size.max(size);
            self.import_return_pointer_area_align =
                self.import_return_pointer_area_align.max(align);
            uwrite!(self.src, "let ptr{tmp} = ret_area.as_mut_ptr() as i32;");
        } else {
            self.gen.return_pointer_area_size = self.gen.return_pointer_area_size.max(size);
            self.gen.return_pointer_area_align = self.gen.return_pointer_area_align.max(align);
            uwriteln!(self.src, "let ptr{tmp} = _RET_AREA.0.as_mut_ptr() as i32;");
        }
        format!("ptr{}", tmp)
    }

    fn sizes(&self) -> &SizeAlign {
        &self.gen.sizes
    }

    fn is_list_canonical(&self, resolve: &Resolve, ty: &Type) -> bool {
        resolve.all_bits_valid(ty)
    }

    fn emit(
        &mut self,
        _resolve: &Resolve,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
        };

        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(format!("{}i32", val)),
            Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("0i32".to_string()),
                        WasmType::I64 => results.push("0i64".to_string()),
                        WasmType::F32 => results.push("0.0f32".to_string()),
                        WasmType::F64 => results.push("0.0f64".to_string()),
                    }
                }
            }

            Instruction::I64FromU64 | Instruction::I64FromS64 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_i64({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }
            Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32
            | Instruction::I32FromS32 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_i32({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }

            Instruction::F32FromFloat32 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_f32({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }
            Instruction::F64FromFloat64 => {
                let s = operands.pop().unwrap();
                results.push(format!(
                    "{rt}::as_f64({s})",
                    rt = self.gen.gen.runtime_path()
                ));
            }
            Instruction::Float32FromF32
            | Instruction::Float64FromF64
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }
            Instruction::S8FromI32 => top_as("i8"),
            Instruction::U8FromI32 => top_as("u8"),
            Instruction::S16FromI32 => top_as("i16"),
            Instruction::U16FromI32 => top_as("u16"),
            Instruction::U32FromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),
            Instruction::CharFromI32 => {
                results.push(format!(
                    "{{
                        #[cfg(not(debug_assertions))]
                        {{ ::core::char::from_u32_unchecked({} as u32) }}
                        #[cfg(debug_assertions)]
                        {{ ::core::char::from_u32({} as u32).unwrap() }}
                    }}",
                    operands[0], operands[0]
                ));
            }

            Instruction::Bitcasts { casts } => {
                wit_bindgen_rust_lib::bitcast(casts, operands, results)
            }

            Instruction::I32FromBool => {
                results.push(format!("match {} {{ true => 1, false => 0 }}", operands[0]));
            }
            Instruction::BoolFromI32 => {
                results.push(format!(
                    "{{
                        #[cfg(not(debug_assertions))]
                        {{ ::core::mem::transmute::<u8, bool>({} as u8) }}
                        #[cfg(debug_assertions)]
                        {{
                            match {} {{
                                0 => false,
                                1 => true,
                                _ => panic!(\"invalid bool discriminant\"),
                            }}
                        }}
                    }}",
                    operands[0], operands[0],
                ));
            }

            Instruction::FlagsLower { flags, .. } => {
                let tmp = self.tmp();
                self.push_str(&format!("let flags{} = {};\n", tmp, operands[0]));
                for i in 0..flags.repr().count() {
                    results.push(format!("(flags{}.bits() >> {}) as i32", tmp, i * 32));
                }
            }
            Instruction::FlagsLift { flags, ty, .. } => {
                let repr = RustFlagsRepr::new(flags);
                let name = self.gen.type_path(*ty, true);
                let mut result = format!("{name}::empty()");
                for (i, op) in operands.iter().enumerate() {
                    result.push_str(&format!(
                        " | {name}::from_bits_retain((({op} as {repr}) << {}) as _)",
                        i * 32
                    ));
                }
                results.push(result);
            }

            Instruction::RecordLower { ty, record, .. } => {
                self.record_lower(*ty, record, &operands[0], results);
            }
            Instruction::RecordLift { ty, record, .. } => {
                self.record_lift(*ty, record, operands, results);
            }

            Instruction::TupleLower { tuple, .. } => {
                self.tuple_lower(tuple, &operands[0], results);
            }
            Instruction::TupleLift { .. } => {
                self.tuple_lift(operands, results);
            }

            Instruction::VariantPayloadName => results.push("e".to_string()),

            Instruction::VariantLower {
                variant,
                results: result_types,
                ty,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                self.let_results(result_types.len(), results);
                let op0 = &operands[0];
                self.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lower(*ty);
                for (case, block) in variant.cases.iter().zip(blocks) {
                    let case_name = case.name.to_upper_camel_case();
                    self.push_str(&format!("{name}::{case_name}"));
                    if case.ty.is_some() {
                        self.push_str(&format!("(e) => {block},\n"));
                    } else {
                        self.push_str(&format!(" => {{\n{block}\n}}\n"));
                    }
                }
                self.push_str("};\n");
            }

            Instruction::VariantLift {
                name, variant, ty, ..
            } => {
                let mut result = String::new();
                result.push_str("{");

                let named_enum = variant.cases.iter().all(|c| c.ty.is_none());
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - variant.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];

                if named_enum {
                    // In unchecked mode when this type is a named enum then we know we
                    // defined the type so we can transmute directly into it.
                    result.push_str("#[cfg(not(debug_assertions))]");
                    result.push_str("{");
                    result.push_str("::core::mem::transmute::<_, ");
                    result.push_str(&name.to_upper_camel_case());
                    result.push_str(">(");
                    result.push_str(op0);
                    result.push_str(" as ");
                    result.push_str(int_repr(variant.tag()));
                    result.push_str(")");
                    result.push_str("}");
                }

                if named_enum {
                    result.push_str("#[cfg(debug_assertions)]");
                }
                result.push_str("{");
                result.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lift(*ty);
                for (i, (case, block)) in variant.cases.iter().zip(blocks).enumerate() {
                    let pat = i.to_string();
                    let block = if case.ty.is_some() {
                        format!("({block})")
                    } else {
                        String::new()
                    };
                    let case = case.name.to_upper_camel_case();
                    if i == variant.cases.len() - 1 {
                        result.push_str("#[cfg(debug_assertions)]");
                        result.push_str(&format!("{pat} => {name}::{case}{block},\n"));
                        result.push_str("#[cfg(not(debug_assertions))]");
                        result.push_str(&format!("_ => {name}::{case}{block},\n"));
                    } else {
                        result.push_str(&format!("{pat} => {name}::{case}{block},\n"));
                    }
                }
                result.push_str("#[cfg(debug_assertions)]");
                result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                result.push_str("}");
                result.push_str("}");

                result.push_str("}");
                results.push(result);
            }

            Instruction::UnionLower {
                union,
                results: result_types,
                ty,
                ..
            } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                self.let_results(result_types.len(), results);
                let op0 = &operands[0];
                self.push_str(&format!("match {op0} {{\n"));
                let name = self.typename_lower(*ty);
                for (case_name, block) in self.gen.union_case_names(union).into_iter().zip(blocks) {
                    self.push_str(&format!("{name}::{case_name}(e) => {block},\n"));
                }
                self.push_str("};\n");
            }

            Instruction::UnionLift { union, ty, .. } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - union.cases.len()..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                let mut result = format!("match {op0} {{\n");
                for (i, (case_name, block)) in self
                    .gen
                    .union_case_names(union)
                    .into_iter()
                    .zip(blocks)
                    .enumerate()
                {
                    let pat = i.to_string();
                    let name = self.typename_lift(*ty);
                    if i == union.cases.len() - 1 {
                        result.push_str("#[cfg(debug_assertions)]");
                        result.push_str(&format!("{pat} => {name}::{case_name}({block}),\n"));
                        result.push_str("#[cfg(not(debug_assertions))]");
                        result.push_str(&format!("_ => {name}::{case_name}({block}),\n"));
                    } else {
                        result.push_str(&format!("{pat} => {name}::{case_name}({block}),\n"));
                    }
                }
                result.push_str("#[cfg(debug_assertions)]");
                result.push_str("_ => panic!(\"invalid union discriminant\"),\n");
                result.push_str("}");
                results.push(result);
            }

            Instruction::OptionLower {
                results: result_types,
                ..
            } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                self.let_results(result_types.len(), results);
                let operand = &operands[0];
                self.push_str(&format!(
                    "match {operand} {{
                        Some(e) => {some},
                        None => {{\n{none}\n}},
                    }};"
                ));
            }

            Instruction::OptionLift { .. } => {
                let some = self.blocks.pop().unwrap();
                let none = self.blocks.pop().unwrap();
                assert_eq!(none, "()");
                let operand = &operands[0];
                results.push(format!(
                    "match {operand} {{
                        0 => None,
                        1 => Some({some}),
                        #[cfg(not(debug_assertions))]
                        _ => ::core::hint::unreachable_unchecked(),
                        #[cfg(debug_assertions)]
                        _ => panic!(\"invalid enum discriminant\"),
                    }}"
                ));
            }

            Instruction::ResultLower {
                results: result_types,
                result,
                ..
            } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                self.let_results(result_types.len(), results);
                let operand = &operands[0];
                let ok_binding = if result.ok.is_some() { "e" } else { "_" };
                let err_binding = if result.err.is_some() { "e" } else { "_" };
                self.push_str(&format!(
                    "match {operand} {{
                        Ok({ok_binding}) => {{ {ok} }},
                        Err({err_binding}) => {{ {err} }},
                    }};"
                ));
            }

            Instruction::ResultLift { .. } => {
                let err = self.blocks.pop().unwrap();
                let ok = self.blocks.pop().unwrap();
                let operand = &operands[0];
                results.push(format!(
                    "match {operand} {{
                        0 => Ok({ok}),
                        1 => Err({err}),
                        #[cfg(not(debug_assertions))]
                        _ => ::core::hint::unreachable_unchecked(),
                        #[cfg(debug_assertions)]
                        _ => panic!(\"invalid enum discriminant\"),
                    }}"
                ));
            }

            Instruction::EnumLower { enum_, ty, .. } => {
                let mut result = format!("match {} {{\n", operands[0]);
                let name = self.gen.type_path(*ty, true);
                for (i, case) in enum_.cases.iter().enumerate() {
                    let case = case.name.to_upper_camel_case();
                    result.push_str(&format!("{name}::{case} => {i},\n"));
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::EnumLift { enum_, ty, .. } => {
                let mut result = String::new();
                result.push_str("{");

                // In checked mode do a `match`.
                result.push_str("#[cfg(debug_assertions)]");
                result.push_str("{");
                result.push_str("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                let name = self.gen.type_path(*ty, true);
                for (i, case) in enum_.cases.iter().enumerate() {
                    let case = case.name.to_upper_camel_case();
                    result.push_str(&format!("{i} => {name}::{case},\n"));
                }
                result.push_str("_ => panic!(\"invalid enum discriminant\"),\n");
                result.push_str("}");
                result.push_str("}");

                // In unchecked mode when this type is a named enum then we know we
                // defined the type so we can transmute directly into it.
                result.push_str("#[cfg(not(debug_assertions))]");
                result.push_str("{");
                result.push_str("::core::mem::transmute::<_, ");
                result.push_str(&self.gen.type_path(*ty, true));
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push_str(" as ");
                result.push_str(int_repr(enum_.tag()));
                result.push_str(")");
                result.push_str("}");

                result.push_str("}");
                results.push(result);
            }

            Instruction::ListCanonLower { realloc, .. } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = operands.pop().unwrap();
                    self.push_str(&format!("let {} = ({}).into_boxed_slice();\n", val, op0));
                }
                self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, val));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, val));
                if realloc.is_some() {
                    self.push_str(&format!("::core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListCanonLift { .. } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                results.push(result);
            }

            Instruction::StringLower { realloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if realloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = format!("{}.into_bytes()", operands[0]);
                    self.push_str(&format!("let {} = ({}).into_boxed_slice();\n", val, op0));
                }
                self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, val));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, val));
                if realloc.is_some() {
                    self.push_str(&format!("::core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::StringLift => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                if self.gen.gen.opts.raw_strings {
                    results.push(result);
                } else {
                    let mut converted = String::new();
                    converted.push_str("{");

                    converted.push_str("#[cfg(not(debug_assertions))]");
                    converted.push_str("{");
                    converted.push_str(&format!("String::from_utf8_unchecked({})", result));
                    converted.push_str("}");

                    converted.push_str("#[cfg(debug_assertions)]");
                    converted.push_str("{");
                    converted.push_str(&format!("String::from_utf8({}).unwrap()", result));
                    converted.push_str("}");

                    converted.push_str("}");
                    results.push(converted);
                }
            }

            Instruction::ListLower { element, realloc } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{tmp}");
                let result = format!("result{tmp}");
                let layout = format!("layout{tmp}");
                let len = format!("len{tmp}");
                self.push_str(&format!(
                    "let {vec} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!("let {len} = {vec}.len() as i32;\n"));
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                self.push_str(&format!(
                    "let {layout} = alloc::Layout::from_size_align_unchecked({vec}.len() * {size}, {align});\n",
                ));
                self.push_str(&format!(
                    "let {result} = if {layout}.size() != 0\n{{\nlet ptr = alloc::alloc({layout});\n",
                ));
                self.push_str(&format!(
                    "if ptr.is_null()\n{{\nalloc::handle_alloc_error({layout});\n}}\nptr\n}}",
                ));
                self.push_str(&format!("else {{\n::core::ptr::null_mut()\n}};\n",));
                self.push_str(&format!("for (i, e) in {vec}.into_iter().enumerate() {{\n",));
                self.push_str(&format!(
                    "let base = {result} as i32 + (i as i32) * {size};\n",
                ));
                self.push_str(&body);
                self.push_str("}\n");
                results.push(format!("{result} as i32"));
                results.push(len);

                if realloc.is_none() {
                    // If an allocator isn't requested then we must clean up the
                    // allocation ourselves since our callee isn't taking
                    // ownership.
                    self.cleanup.push((result, layout));
                }
            }

            Instruction::ListLift { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{tmp}");
                let base = format!("base{tmp}");
                let result = format!("result{tmp}");
                self.push_str(&format!(
                    "let {base} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!(
                    "let {len} = {operand1};\n",
                    operand1 = operands[1]
                ));
                self.push_str(&format!(
                    "let mut {result} = Vec::with_capacity({len} as usize);\n",
                ));

                self.push_str("for i in 0..");
                self.push_str(&len);
                self.push_str(" {\n");
                self.push_str("let base = ");
                self.push_str(&base);
                self.push_str(" + i *");
                self.push_str(&size.to_string());
                self.push_str(";\n");
                self.push_str(&result);
                self.push_str(".push(");
                self.push_str(&body);
                self.push_str(");\n");
                self.push_str("}\n");
                results.push(result);
                self.push_str(&format!(
                    "{rt}::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
                ));
            }

            Instruction::IterElem { .. } => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm { name, sig, .. } => {
                let func = self.declare_import(
                    self.gen.wasm_import_module.unwrap(),
                    name,
                    &sig.params,
                    &sig.results,
                );

                // ... then call the function with all our operands
                if sig.results.len() > 0 {
                    self.push_str("let ret = ");
                    results.push("ret".to_string());
                }
                self.push_str(&func);
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");\n");
            }

            Instruction::CallInterface { func, .. } => {
                self.let_results(func.results.len(), results);
                match &func.kind {
                    FunctionKind::Constructor(_)
                    | FunctionKind::Method(_)
                    | FunctionKind::Static(_) => {
                        todo!("implement resources")
                    }
                    FunctionKind::Freestanding => {
                        self.push_str(&format!("T::{}", to_rust_ident(&func.name)));
                    }
                }
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(")");
                self.push_str(";\n");
            }

            Instruction::Return { amt, .. } => {
                self.emit_cleanup();
                match amt {
                    0 => {}
                    1 => {
                        self.push_str(&operands[0]);
                        self.push_str("\n");
                    }
                    _ => {
                        self.push_str("(");
                        self.push_str(&operands.join(", "));
                        self.push_str(")\n");
                    }
                }
            }

            Instruction::I32Load { offset } => {
                results.push(format!("*(({} + {}) as *const i32)", operands[0], offset));
            }
            Instruction::I32Load8U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u16))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i16))",
                    operands[0], offset
                ));
            }
            Instruction::I64Load { offset } => {
                results.push(format!("*(({} + {}) as *const i64)", operands[0], offset));
            }
            Instruction::F32Load { offset } => {
                results.push(format!("*(({} + {}) as *const f32)", operands[0], offset));
            }
            Instruction::F64Load { offset } => {
                results.push(format!("*(({} + {}) as *const f64)", operands[0], offset));
            }
            Instruction::I32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u8) = ({}) as u8;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u16) = ({}) as u16;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }

            Instruction::Malloc { .. } => unimplemented!(),

            Instruction::GuestDeallocate { size, align } => {
                self.push_str(&format!(
                    "{rt}::dealloc({op}, {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
                    op = operands[0]
                ));
            }

            Instruction::GuestDeallocateString => {
                self.push_str(&format!(
                    "{rt}::dealloc({op0}, ({op1}) as usize, 1);\n",
                    rt = self.gen.gen.runtime_path(),
                    op0 = operands[0],
                    op1 = operands[1],
                ));
            }

            Instruction::GuestDeallocateVariant { blocks } => {
                let max = blocks - 1;
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - blocks..)
                    .collect::<Vec<_>>();
                let op0 = &operands[0];
                self.src.push_str(&format!("match {op0} {{\n"));
                for (i, block) in blocks.into_iter().enumerate() {
                    let pat = if i == max {
                        String::from("_")
                    } else {
                        i.to_string()
                    };
                    self.src.push_str(&format!("{pat} => {block},\n"));
                }
                self.src.push_str("}\n");
            }

            Instruction::GuestDeallocateList { element } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size = self.gen.sizes.size(element);
                let align = self.gen.sizes.align(element);
                let len = format!("len{tmp}");
                let base = format!("base{tmp}");
                self.push_str(&format!(
                    "let {base} = {operand0};\n",
                    operand0 = operands[0]
                ));
                self.push_str(&format!(
                    "let {len} = {operand1};\n",
                    operand1 = operands[1]
                ));

                if body != "()" {
                    self.push_str("for i in 0..");
                    self.push_str(&len);
                    self.push_str(" {\n");
                    self.push_str("let base = ");
                    self.push_str(&base);
                    self.push_str(" + i *");
                    self.push_str(&size.to_string());
                    self.push_str(";\n");
                    self.push_str(&body);
                    self.push_str("\n}\n");
                }
                self.push_str(&format!(
                    "{rt}::dealloc({base}, ({len} as usize) * {size}, {align});\n",
                    rt = self.gen.gen.runtime_path(),
                ));
            }

            Instruction::HandleLift { .. } | Instruction::HandleLower { .. } => {
                todo!("implement resources")
            }
        }
    }
}
