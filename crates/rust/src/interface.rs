use crate::bindgen::FunctionBindgen;
use crate::{Direction, ExportKey, Identifier, RustWasm};
use anyhow::Result;
use heck::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::mem;
use wit_bindgen_core::abi::{self, AbiVariant, LiftLower};
use wit_bindgen_core::{uwrite, uwriteln, wit_parser::*, Source, TypeInfo, Types};
use wit_bindgen_rust_lib::{
    dealias, int_repr, to_rust_ident, to_upper_camel_case, wasm_type, FnSig, Ownership,
    RustFlagsRepr, RustGenerator, TypeMode,
};

pub struct InterfaceGenerator<'a> {
    pub src: Source,
    pub(super) identifier: Identifier<'a>,
    pub in_import: bool,
    pub sizes: SizeAlign,
    pub(super) gen: &'a mut RustWasm,
    pub wasm_import_module: Option<&'a str>,
    pub resolve: &'a Resolve,
    pub return_pointer_area_size: usize,
    pub return_pointer_area_align: usize,
}

impl InterfaceGenerator<'_> {
    pub(super) fn generate_exports<'a>(
        &mut self,
        export_key: &ExportKey,
        interface_name: Option<&WorldKey>,
        funcs: impl Iterator<Item = &'a Function> + Clone,
    ) -> Result<()> {
        let mut traits = BTreeMap::new();

        for func in funcs {
            if self.gen.skip.contains(&func.name) {
                continue;
            }

            // First generate the exported function which performs lift/lower
            // operations and delegates to a trait (that doesn't exist just yet).
            self.src.push_str("const _: () = {\n");
            self.generate_guest_export(func, interface_name);
            self.src.push_str("};\n");

            // Next generate a trait signature for this method and insert it
            // into `traits`. Note that `traits` will have a trait-per-resource.
            let (trait_name, local_impl_name, export_key) = match func.kind {
                FunctionKind::Freestanding => (
                    "Guest".to_string(),
                    "_GuestImpl".to_string(),
                    export_key.clone(),
                ),
                FunctionKind::Method(id)
                | FunctionKind::Constructor(id)
                | FunctionKind::Static(id) => {
                    let resource_name = self.resolve.types[id].name.as_deref().unwrap();
                    let camel = resource_name.to_upper_camel_case();
                    let trait_name = format!("Guest{camel}");
                    let export_key = match export_key {
                        ExportKey::World => unimplemented!("exported world resources"),
                        ExportKey::Name(path) => ExportKey::Name(format!("{path}/{resource_name}")),
                    };
                    let local_impl_name = format!("_{camel}Impl");
                    (trait_name, local_impl_name, export_key)
                }
            };

            let (_, _, methods) =
                traits
                    .entry(export_key)
                    .or_insert((trait_name, local_impl_name, Vec::new()));
            let prev = mem::take(&mut self.src);
            let mut sig = FnSig {
                use_item_name: true,
                private: true,
                ..Default::default()
            };
            if let FunctionKind::Method(_) = &func.kind {
                sig.self_arg = Some("&self".into());
                sig.self_is_first_param = true;
            }
            self.print_signature(func, TypeMode::Owned, &sig);
            self.src.push_str(";\n");
            let trait_method = mem::replace(&mut self.src, prev);
            methods.push(trait_method);
        }

        // Once all the traits have been assembled then they can be emitted.
        //
        // Additionally alias the user-configured item for each trait here as
        // there's only one implementation of this trait and it must be
        // pre-configured.
        for (export_key, (trait_name, local_impl_name, methods)) in traits {
            let impl_name = self.gen.lookup_export(&export_key)?;
            let path_to_root = self.path_to_root();
            uwriteln!(
                self.src,
                "use {path_to_root}{impl_name} as {local_impl_name};"
            );

            uwriteln!(self.src, "pub trait {trait_name} {{");
            for method in methods {
                self.src.push_str(&method);
            }
            uwriteln!(self.src, "}}");
        }

        Ok(())
    }

    pub fn generate_imports<'a>(&mut self, funcs: impl Iterator<Item = &'a Function>) {
        for func in funcs {
            self.generate_guest_import(func);
        }
    }

    pub fn finish(&mut self) -> String {
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

    fn path_to_root(&self) -> String {
        let mut path_to_root = String::new();

        if let Identifier::Interface(_, key) = self.identifier {
            // Escape the submodule for this interface
            path_to_root.push_str("super::");

            // Escape the `exports` top-level submodule
            if !self.in_import {
                path_to_root.push_str("super::");
            }

            // Escape the namespace/package submodules for interface-based ids
            match key {
                WorldKey::Name(_) => {}
                WorldKey::Interface(_) => {
                    path_to_root.push_str("super::super::");
                }
            }
        }
        path_to_root
    }

    pub fn start_append_submodule(&mut self, name: &WorldKey) -> (String, Option<PackageName>) {
        let snake = match name {
            WorldKey::Name(name) => to_rust_ident(name),
            WorldKey::Interface(id) => {
                to_rust_ident(self.resolve.interfaces[*id].name.as_ref().unwrap())
            }
        };
        let pkg = match name {
            WorldKey::Name(_) => None,
            WorldKey::Interface(id) => {
                let pkg = self.resolve.interfaces[*id].package.unwrap();
                Some(self.resolve.packages[pkg].name.clone())
            }
        };
        if let Identifier::Interface(id, _) = self.identifier {
            let mut path = String::new();
            if !self.in_import {
                path.push_str("exports::");
            }
            if let Some(name) = &pkg {
                path.push_str(&format!(
                    "{}::{}::",
                    name.namespace.to_snake_case(),
                    name.name.to_snake_case()
                ));
            }
            path.push_str(&snake);
            self.gen.interface_names.insert(id, path);
        }
        (snake, pkg)
    }

    pub fn finish_append_submodule(mut self, snake: &str, pkg: Option<PackageName>) {
        let module = self.finish();
        let path_to_root = self.path_to_root();
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

        let mut sig = FnSig::default();
        let param_mode = TypeMode::AllBorrowed("'_");
        match func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Method(id) | FunctionKind::Static(id) | FunctionKind::Constructor(id) => {
                let name = self.resolve.types[id].name.as_ref().unwrap();
                let name = to_upper_camel_case(name);
                uwriteln!(self.src, "impl {name} {{");
                sig.use_item_name = true;
                if let FunctionKind::Method(_) = &func.kind {
                    sig.self_arg = Some("&self".into());
                    sig.self_is_first_param = true;
                }
            }
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
        abi::call(
            f.gen.resolve,
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

        match func.kind {
            FunctionKind::Freestanding => {}
            FunctionKind::Method(_) | FunctionKind::Static(_) | FunctionKind::Constructor(_) => {
                self.src.push_str("}\n");
            }
        }
    }

    fn generate_guest_export(&mut self, func: &Function, interface_name: Option<&WorldKey>) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let name_snake = func.name.to_snake_case().replace('.', "_");
        let wasm_module_export_name = interface_name.map(|k| self.resolve.name_world_key(k));
        let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
        let export_name = func.core_export_name(wasm_module_export_name.as_deref());
        uwrite!(
            self.src,
            "
                #[doc(hidden)]
                #[export_name = \"{export_prefix}{export_name}\"]
                #[allow(non_snake_case)]
                unsafe extern \"C\" fn __export_{name_snake}(\
            ",
        );

        let sig = self.resolve.wasm_signature(AbiVariant::GuestExport, func);
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            uwrite!(self.src, "{name}: {},", wasm_type(*param));
            params.push(name);
        }
        self.src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                uwrite!(self.src, " -> {}", wasm_type(sig.results[0]));
            }
            _ => unimplemented!(),
        }

        self.push_str(" {");

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

        let mut f = FunctionBindgen::new(self, params);
        abi::call(
            f.gen.resolve,
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

        if abi::guest_export_needs_post_return(self.resolve, func) {
            let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
            uwrite!(
                self.src,
                "
                    const _: () = {{
                    #[doc(hidden)]
                    #[export_name = \"{export_prefix}cabi_post_{export_name}\"]
                    #[allow(non_snake_case)]
                    unsafe extern \"C\" fn __post_return_{name_snake}(\
                "
            );
            let mut params = Vec::new();
            for (i, result) in sig.results.iter().enumerate() {
                let name = format!("arg{}", i);
                uwrite!(self.src, "{name}: {},", wasm_type(*result));
                params.push(name);
            }
            self.src.push_str(") {\n");

            let mut f = FunctionBindgen::new(self, params);
            abi::post_return(f.gen.resolve, func, &mut f);
            let FunctionBindgen {
                needs_cleanup_list,
                src,
                ..
            } = f;
            assert!(!needs_cleanup_list);
            self.src.push_str(&String::from(src));
            self.src.push_str("}\n");
            self.src.push_str("};\n");
        }
    }

    pub fn generate_stub(
        &mut self,
        resource: Option<TypeId>,
        pkg: Option<&PackageName>,
        name: &str,
        in_interface: bool,
        funcs: &[&Function],
    ) {
        let path = if let Some(pkg) = pkg {
            format!(
                "{}::{}::{}",
                to_rust_ident(&pkg.namespace),
                to_rust_ident(&pkg.name),
                to_rust_ident(name),
            )
        } else {
            to_rust_ident(name)
        };

        let name = resource
            .map(|ty| {
                format!(
                    "Guest{}",
                    self.resolve.types[ty]
                        .name
                        .as_deref()
                        .unwrap()
                        .to_upper_camel_case()
                )
            })
            .unwrap_or_else(|| "Guest".to_string());

        let qualified_name = if in_interface {
            format!("exports::{path}::{name}")
        } else {
            name
        };

        uwriteln!(self.src, "impl {qualified_name} for Stub {{");

        for &func in funcs {
            if self.gen.skip.contains(&func.name) {
                continue;
            }
            let mut sig = FnSig {
                use_item_name: true,
                private: true,
                ..Default::default()
            };
            if let FunctionKind::Method(_) = &func.kind {
                sig.self_arg = Some("&self".into());
                sig.self_is_first_param = true;
            }
            self.print_signature(func, TypeMode::Owned, &sig);
            self.src.push_str("{ unreachable!() }\n");
        }

        self.src.push_str("}\n");
    }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn ownership(&self) -> Ownership {
        self.gen.opts.ownership
    }

    fn additional_derives(&self) -> &[String] {
        &self.gen.opts.additional_derive_attributes
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let mut path = String::new();
        if let Identifier::Interface(cur, name) = self.identifier {
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
        path.push_str(name);
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

    fn is_exported_resource(&self, mut ty: TypeId) -> bool {
        loop {
            let def = &self.resolve.types[ty];
            if let TypeOwner::World(_) = &def.owner {
                // Worlds cannot export types of any kind as of this writing.
                return false;
            }
            match &def.kind {
                TypeDefKind::Type(Type::Id(id)) => ty = *id,
                _ => break,
            }
        }

        matches!(
            self.gen.resources.get(&ty).map(|info| info.direction),
            Some(Direction::Export)
        )
    }

    fn mark_resource_owned(&mut self, resource: TypeId) {
        self.gen
            .resources
            .entry(dealias(self.resolve, resource))
            .or_default()
            .owned = true;
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

    fn type_resource(&mut self, id: TypeId, name: &str, docs: &Docs) {
        let entry = self
            .gen
            .resources
            .entry(dealias(self.resolve, id))
            .or_default();
        if !self.in_import {
            entry.direction = Direction::Export;
        }
        self.rustdoc(docs);
        let camel = to_upper_camel_case(name);
        let rt = self.gen.runtime_path();

        let wasm_import_module = if self.in_import {
            // Imported resources are a simple wrapper around `Resource<T>` in
            // the `wit-bindgen` crate.
            uwriteln!(
                self.src,
                r#"
                    #[derive(Debug)]
                    pub struct {camel} {{
                        handle: {rt}::Resource<{camel}>,
                    }}

                    impl {camel} {{
                        #[doc(hidden)]
                        pub unsafe fn from_handle(handle: u32) -> Self {{
                            Self {{
                                handle: {rt}::Resource::from_handle(handle),
                            }}
                        }}

                        #[doc(hidden)]
                        pub fn into_handle(self) -> u32 {{
                            {rt}::Resource::into_handle(self.handle)
                        }}

                        #[doc(hidden)]
                        pub fn handle(&self) -> u32 {{
                            {rt}::Resource::handle(&self.handle)
                        }}
                    }}
                "#
            );
            self.wasm_import_module.unwrap().to_string()
        } else {
            // Exported resources are represented as `Resource<T>` as opposed
            // to being wrapped like imported resources.
            //
            // An `Own` typedef is available for the `Resource<T>` type though.
            //
            // Note that the actual name `{camel}` is defined here though as
            // an alias of the type this is implemented by as configured by the
            // `exports` configuration by the user.
            let export_prefix = self.gen.opts.export_prefix.as_deref().unwrap_or("");
            let module = match self.identifier {
                Identifier::Interface(_, key) => self.resolve.name_world_key(key),
                Identifier::World(_) => unimplemented!("resource exports from worlds"),
            };
            let export_key = ExportKey::Name(format!("{module}/{name}"));
            // NB: errors are ignored here since they'll generate an error
            // through the `generate_exports` method above.
            let impl_name = self
                .gen
                .lookup_export(&export_key)
                .unwrap_or_else(|_| "ERROR".to_string());
            let path_to_root = self.path_to_root();
            uwriteln!(
                self.src,
                r#"
                    pub use {path_to_root}{impl_name} as {camel};
                    const _: () = {{
                        #[doc(hidden)]
                        #[export_name = "{export_prefix}{module}#[dtor]{name}"]
                        #[allow(non_snake_case)]
                        unsafe extern "C" fn dtor(rep: usize) {{
                            {rt}::Resource::<{camel}>::dtor(rep)
                        }}
                    }};
                    unsafe impl {rt}::RustResource for {camel} {{
                        unsafe fn new(rep: usize) -> u32 {{
                            #[cfg(not(target_arch = "wasm32"))]
                            unreachable!();

                            #[cfg(target_arch = "wasm32")]
                            {{
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-new]{name}"]
                                    fn new(_: usize) -> u32;
                                }}
                                new(rep)
                            }}
                        }}

                        unsafe fn rep(handle: u32) -> usize {{
                            #[cfg(not(target_arch = "wasm32"))]
                            unreachable!();

                            #[cfg(target_arch = "wasm32")]
                            {{
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-rep]{name}"]
                                    fn rep(_: u32) -> usize;
                                }}
                                rep(handle)
                            }}
                        }}
                    }}
                    pub type Own{camel} = {rt}::Resource<{camel}>;
                "#
            );
            format!("[export]{module}")
        };

        uwriteln!(
            self.src,
            r#"
                unsafe impl {rt}::WasmResource for {camel} {{
                     #[inline]
                     unsafe fn drop(handle: u32) {{
                         #[cfg(not(target_arch = "wasm32"))]
                         unreachable!();

                         #[cfg(target_arch = "wasm32")]
                         {{
                             #[link(wasm_import_module = "{wasm_import_module}")]
                             extern "C" {{
                                 #[link_name = "[resource-drop]{name}"]
                                 fn drop(_: u32);
                             }}

                             drop(handle);
                         }}
                     }}
                }}
            "#
        );
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_typedef_tuple(id, tuple, docs);
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.src.push_str(&format!(
            "{bitflags}::bitflags! {{\n",
            bitflags = self.gen.bitflags_path()
        ));
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

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        self.print_typedef_option(id, payload, docs);
    }

    fn type_result(&mut self, id: TypeId, _name: &str, result: &Result_, docs: &Docs) {
        self.print_typedef_result(id, result, docs);
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs, &[], Box::new(|_| String::new()));

        let name = to_upper_camel_case(name);
        let mut cases = String::new();
        let repr = int_repr(enum_.tag());
        for (i, case) in enum_.cases.iter().enumerate() {
            let case = case.name.to_upper_camel_case();
            cases.push_str(&format!("{i} => {name}::{case},\n"));
        }
        uwriteln!(
            self.src,
            r#"
                impl {name} {{
                    pub(crate) unsafe fn _lift(val: {repr}) -> {name} {{
                        if !cfg!(debug_assertions) {{
                            return ::core::mem::transmute(val);
                        }}

                        match val {{
                            {cases}
                            _ => panic!("invalid enum discriminant"),
                        }}
                    }}
                }}
            "#
        );
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
