use crate::bindgen::FunctionBindgen;
use crate::{
    int_repr, to_rust_ident, to_upper_camel_case, wasm_type, Direction, ExportKey, FnSig,
    Identifier, InterfaceName, Ownership, RustFlagsRepr, RustWasm, TypeMode,
};
use anyhow::Result;
use heck::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::mem;
use wit_bindgen_core::abi::{self, AbiVariant, LiftLower};
use wit_bindgen_core::{dealias, uwrite, uwriteln, wit_parser::*, Source, TypeInfo};

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
    fn export_key(&self, item: Option<&str>) -> ExportKey {
        let base = match self.identifier {
            Identifier::World(_) => ExportKey::World,
            Identifier::Interface(_, WorldKey::Name(n)) => ExportKey::Name(n.to_string()),

            // If an interface belongs to a package with a version then `id_of`
            // will print the version, but versions are onerous to keep in sync
            // and write down everywhere. In lieu of proliferating the
            // requirement of everyone always thinking about versions this
            // will attempt to drop the version if it can unambiguously be
            // dropped.
            //
            // If this interface belongs to a package with a version, and there
            // is no other package of the same name/namespace, then drop the
            // version from the export key.
            Identifier::Interface(_, WorldKey::Interface(n)) => {
                let iface = &self.resolve.interfaces[*n];
                let package = iface.package.unwrap();
                let package_name = &self.resolve.packages[package].name;
                if package_name.version.is_some()
                    && self
                        .resolve
                        .package_names
                        .iter()
                        .filter(|(name, _)| {
                            package_name.name == name.name
                                && package_name.namespace == name.namespace
                        })
                        .count()
                        == 1
                {
                    ExportKey::Name(format!(
                        "{}:{}/{}",
                        package_name.namespace,
                        package_name.name,
                        iface.name.as_ref().unwrap()
                    ))
                } else {
                    ExportKey::Name(self.resolve.id_of(*n).unwrap())
                }
            }
        };
        match item {
            Some(item) => match base {
                ExportKey::World => unimplemented!("item projected from world interface"),
                ExportKey::Name(name) => ExportKey::Name(format!("{name}/{item}")),
            },
            None => base,
        }
    }

    pub(super) fn generate_exports<'a>(
        &mut self,
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
            self.generate_guest_export(func);
            self.src.push_str("};\n");

            // Next generate a trait signature for this method and insert it
            // into `traits`. Note that `traits` will have a trait-per-resource.
            let (trait_name, local_impl_name, export_key) = match func.kind {
                FunctionKind::Freestanding => (
                    "Guest".to_string(),
                    "_GuestImpl".to_string(),
                    self.export_key(None),
                ),
                FunctionKind::Method(id)
                | FunctionKind::Constructor(id)
                | FunctionKind::Static(id) => {
                    let resource_name = self.resolve.types[id].name.as_deref().unwrap();
                    let camel = resource_name.to_upper_camel_case();
                    let trait_name = format!("Guest{camel}");
                    let export_key = self.export_key(Some(&resource_name));
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

    pub fn start_append_submodule(&mut self, name: &WorldKey) -> (String, Vec<String>) {
        let snake = match name {
            WorldKey::Name(name) => to_rust_ident(name),
            WorldKey::Interface(id) => {
                to_rust_ident(self.resolve.interfaces[*id].name.as_ref().unwrap())
            }
        };
        let module_path = crate::compute_module_path(name, &self.resolve, !self.in_import);
        (snake, module_path)
    }

    pub fn finish_append_submodule(mut self, snake: &str, module_path: Vec<String>) {
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
        map.push((module, module_path))
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
        self.src.push_str("#[allow(unused_unsafe, clippy::all)]\n");
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
            handle_decls,
            ..
        } = f;

        if needs_cleanup_list {
            self.src.push_str("let mut cleanup_list = Vec::new();\n");
        }
        assert!(handle_decls.is_empty());
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

    fn generate_guest_export(&mut self, func: &Function) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        let name_snake = func.name.to_snake_case().replace('.', "_");
        let wasm_module_export_name = match self.identifier {
            Identifier::Interface(_, key) => Some(self.resolve.name_world_key(key)),
            Identifier::World(_) => None,
        };
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
            handle_decls,
            ..
        } = f;
        assert!(!needs_cleanup_list);
        for decl in handle_decls {
            self.src.push_str(&decl);
            self.src.push_str("\n");
        }
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
                handle_decls,
                ..
            } = f;
            assert!(!needs_cleanup_list);
            assert!(handle_decls.is_empty());
            self.src.push_str(&String::from(src));
            self.src.push_str("}\n");
            self.src.push_str("};\n");
        }
    }

    pub fn generate_stub(
        &mut self,
        resource: Option<TypeId>,
        pkg: Option<(String, String)>,
        name: &str,
        in_interface: bool,
        funcs: &[&Function],
    ) {
        let path = if let Some((namespace, pkg_name)) = pkg {
            format!(
                "{}::{}::{}",
                to_rust_ident(&namespace),
                to_rust_ident(&pkg_name),
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

    fn rustdoc(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.trim().lines() {
            self.push_str("/// ");
            self.push_str(line);
            self.push_str("\n");
        }
    }

    fn rustdoc_params(&mut self, docs: &[(String, Type)], header: &str) {
        let _ = (docs, header);
        // let docs = docs
        //     .iter()
        //     .filter(|param| param.docs.trim().len() > 0)
        //     .collect::<Vec<_>>();
        // if docs.len() == 0 {
        //     return;
        // }

        // self.push_str("///\n");
        // self.push_str("/// ## ");
        // self.push_str(header);
        // self.push_str("\n");
        // self.push_str("///\n");

        // for param in docs {
        //     for (i, line) in param.docs.lines().enumerate() {
        //         self.push_str("/// ");
        //         // Currently wasi only has at most one return value, so there's no
        //         // need to indent it or name it.
        //         if header != "Return" {
        //             if i == 0 {
        //                 self.push_str("* `");
        //                 self.push_str(to_rust_ident(param.name.as_str()));
        //                 self.push_str("` - ");
        //             } else {
        //                 self.push_str("  ");
        //             }
        //         }
        //         self.push_str(line);
        //         self.push_str("\n");
        //     }
        // }
    }

    fn print_signature(
        &mut self,
        func: &Function,
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        let params = self.print_docs_and_params(func, param_mode, sig);
        if let FunctionKind::Constructor(_) = &func.kind {
            self.push_str(" -> Self")
        } else {
            self.print_results(&func.results, TypeMode::Owned);
        }
        params
    }

    fn print_docs_and_params(
        &mut self,
        func: &Function,
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        // TODO: re-add this when docs are back
        // self.rustdoc_params(&func.results, "Return");

        if !sig.private {
            self.push_str("pub ");
        }
        if sig.unsafe_ {
            self.push_str("unsafe ");
        }
        if sig.async_ {
            self.push_str("async ");
        }
        self.push_str("fn ");
        let func_name = if sig.use_item_name {
            if let FunctionKind::Constructor(_) = &func.kind {
                "new"
            } else {
                func.item_name()
            }
        } else {
            &func.name
        };
        self.push_str(&to_rust_ident(func_name));
        if let Some(generics) = &sig.generics {
            self.push_str(generics);
        }
        self.push_str("(");
        if let Some(arg) = &sig.self_arg {
            self.push_str(arg);
            self.push_str(",");
        }
        let mut params = Vec::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i == 0 && sig.self_is_first_param {
                params.push("self".to_string());
                continue;
            }
            let name = to_rust_ident(name);
            self.push_str(&name);
            params.push(name);
            self.push_str(": ");
            self.print_ty(param, param_mode);
            self.push_str(",");
        }
        self.push_str(")");
        params
    }

    fn print_results(&mut self, results: &Results, mode: TypeMode) {
        match results.len() {
            0 => {}
            1 => {
                self.push_str(" -> ");
                self.print_ty(results.iter_types().next().unwrap(), mode);
            }
            _ => {
                self.push_str(" -> (");
                for ty in results.iter_types() {
                    self.print_ty(ty, mode);
                    self.push_str(", ")
                }
                self.push_str(")")
            }
        }
    }

    fn print_ty(&mut self, ty: &Type, mode: TypeMode) {
        match ty {
            Type::Id(t) => self.print_tyid(*t, mode),
            Type::Bool => self.push_str("bool"),
            Type::U8 => self.push_str("u8"),
            Type::U16 => self.push_str("u16"),
            Type::U32 => self.push_str("u32"),
            Type::U64 => self.push_str("u64"),
            Type::S8 => self.push_str("i8"),
            Type::S16 => self.push_str("i16"),
            Type::S32 => self.push_str("i32"),
            Type::S64 => self.push_str("i64"),
            Type::Float32 => self.push_str("f32"),
            Type::Float64 => self.push_str("f64"),
            Type::Char => self.push_str("char"),
            Type::String => match mode {
                TypeMode::AllBorrowed(lt) => self.print_borrowed_str(lt),
                TypeMode::Owned | TypeMode::HandlesBorrowed(_) => {
                    if self.gen.opts.raw_strings {
                        self.push_vec_name();
                        self.push_str("::<u8>");
                    } else {
                        self.push_string_name();
                    }
                }
            },
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>, mode: TypeMode) {
        match ty {
            Some(ty) => self.print_ty(ty, mode),
            None => self.push_str("()"),
        }
    }

    pub fn type_path(&self, id: TypeId, owned: bool) -> String {
        self.type_path_with_name(
            id,
            if owned {
                self.result_name(id)
            } else {
                self.param_name(id)
            },
        )
    }

    fn type_path_with_name(&self, id: TypeId, name: String) -> String {
        if let TypeOwner::Interface(id) = self.resolve.types[id].owner {
            if let Some(path) = self.path_to_interface(id) {
                return format!("{path}::{name}");
            }
        }
        name
    }

    fn print_tyid(&mut self, id: TypeId, mode: TypeMode) {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        let ty = &self.resolve.types[id];
        if ty.name.is_some() {
            // If `mode` is borrowed then that means literal ownership of the
            // input type is not necessarily required. In this situation we
            // ideally want to put a `&` in front to statically indicate this.
            // That's not required in all situations however and is only really
            // critical for lists which otherwise would transfer ownership of
            // the allocation to this function.
            //
            // Note, though, that if the type has an `own<T>` inside of it then
            // it is actually required that we take ownership since Rust is
            // losing access to those handles.
            //
            // We also skip borrowing if the type has a lifetime associated with
            // in which case we treated it as already borrowed.
            //
            // Check here if the type has the right shape and if we're in the
            // right mode, and if those conditions are met a lifetime is
            // printed.
            if info.has_list && !info.has_own_handle && lt.is_none() {
                if let TypeMode::AllBorrowed(lt) | TypeMode::HandlesBorrowed(lt) = mode {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                        self.push_str(" ");
                    }
                }
            }
            let name = self.type_path(id, lt.is_none());
            self.push_str(&name);

            // If the type recursively owns data and it's a
            // variant/record/list, then we need to place the
            // lifetime parameter on the type as well.
            if (info.has_list || info.has_borrow_handle)
                && !info.has_own_handle
                && needs_generics(self.resolve, &ty.kind)
            {
                self.print_generics(lt);
            }

            return;

            fn needs_generics(resolve: &Resolve, ty: &TypeDefKind) -> bool {
                match ty {
                    TypeDefKind::Variant(_)
                    | TypeDefKind::Record(_)
                    | TypeDefKind::Option(_)
                    | TypeDefKind::Result(_)
                    | TypeDefKind::Future(_)
                    | TypeDefKind::Stream(_)
                    | TypeDefKind::List(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Tuple(_) => true,
                    TypeDefKind::Type(Type::Id(t)) => {
                        needs_generics(resolve, &resolve.types[*t].kind)
                    }
                    TypeDefKind::Type(Type::String) => true,
                    TypeDefKind::Handle(Handle::Borrow(_)) => true,
                    TypeDefKind::Resource | TypeDefKind::Handle(_) | TypeDefKind::Type(_) => false,
                    TypeDefKind::Unknown => unreachable!(),
                }
            }
        }

        match &ty.kind {
            TypeDefKind::List(t) => self.print_list(t, mode),

            TypeDefKind::Option(t) => {
                self.push_str("Option<");
                self.print_ty(t, mode);
                self.push_str(">");
            }

            TypeDefKind::Result(r) => {
                self.push_str("Result<");
                self.print_optional_ty(r.ok.as_ref(), mode);
                self.push_str(",");
                self.print_optional_ty(r.err.as_ref(), mode);
                self.push_str(">");
            }

            TypeDefKind::Variant(_) => panic!("unsupported anonymous variant"),

            // Tuple-like records are mapped directly to Rust tuples of
            // types. Note the trailing comma after each member to
            // appropriately handle 1-tuples.
            TypeDefKind::Tuple(t) => {
                self.push_str("(");
                for ty in t.types.iter() {
                    self.print_ty(ty, mode);
                    self.push_str(",");
                }
                self.push_str(")");
            }
            TypeDefKind::Resource => {
                panic!("unsupported anonymous type reference: resource")
            }
            TypeDefKind::Record(_) => {
                panic!("unsupported anonymous type reference: record")
            }
            TypeDefKind::Flags(_) => {
                panic!("unsupported anonymous type reference: flags")
            }
            TypeDefKind::Enum(_) => {
                panic!("unsupported anonymous type reference: enum")
            }
            TypeDefKind::Future(ty) => {
                self.push_str("Future<");
                self.print_optional_ty(ty.as_ref(), mode);
                self.push_str(">");
            }
            TypeDefKind::Stream(stream) => {
                self.push_str("Stream<");
                self.print_optional_ty(stream.element.as_ref(), mode);
                self.push_str(",");
                self.print_optional_ty(stream.end.as_ref(), mode);
                self.push_str(">");
            }

            TypeDefKind::Handle(Handle::Own(ty)) => {
                self.mark_resource_owned(*ty);
                self.print_ty(&Type::Id(*ty), mode);
            }

            TypeDefKind::Handle(Handle::Borrow(ty)) => {
                self.push_str("&");
                if let TypeMode::AllBorrowed(lt) | TypeMode::HandlesBorrowed(lt) = mode {
                    if lt != "'_" {
                        self.push_str(lt);
                        self.push_str(" ");
                    }
                }
                if self.is_exported_resource(*ty) {
                    self.push_str(
                        &self.type_path_with_name(
                            *ty,
                            self.resolve.types[*ty]
                                .name
                                .as_deref()
                                .unwrap()
                                .to_upper_camel_case(),
                        ),
                    );
                } else {
                    self.print_ty(&Type::Id(*ty), mode);
                }
            }

            TypeDefKind::Type(t) => self.print_ty(t, mode),

            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn print_list(&mut self, ty: &Type, mode: TypeMode) {
        let next_mode = if matches!(self.gen.opts.ownership, Ownership::Owning) {
            if let TypeMode::HandlesBorrowed(_) = mode {
                mode
            } else {
                TypeMode::Owned
            }
        } else {
            mode
        };
        // Lists with `own` handles must always be owned
        let mode = match *ty {
            Type::Id(id) if self.info(id).has_own_handle => TypeMode::Owned,
            _ => mode,
        };
        match mode {
            TypeMode::AllBorrowed(lt) => {
                self.print_borrowed_slice(false, ty, lt, next_mode);
            }
            TypeMode::Owned | TypeMode::HandlesBorrowed(_) => {
                self.push_vec_name();
                self.push_str("::<");
                self.print_ty(ty, next_mode);
                self.push_str(">");
            }
        }
    }

    fn print_rust_slice(&mut self, mutbl: bool, ty: &Type, lifetime: &'static str, mode: TypeMode) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if mutbl {
            self.push_str(" mut ");
        }
        self.push_str("[");
        self.print_ty(ty, mode);
        self.push_str("]");
    }

    fn print_generics(&mut self, lifetime: Option<&str>) {
        if lifetime.is_none() {
            return;
        }
        self.push_str("<");
        if let Some(lt) = lifetime {
            self.push_str(lt);
            self.push_str(",");
        }
        self.push_str(">");
    }

    fn int_repr(&mut self, repr: Int) {
        self.push_str(int_repr(repr));
    }

    fn modes_of(&self, ty: TypeId) -> Vec<(String, TypeMode)> {
        let info = self.info(ty);
        // If this type isn't actually used, no need to generate it.
        if !info.owned && !info.borrowed {
            return Vec::new();
        }
        let mut result = Vec::new();

        // Prioritize generating an "owned" type. This is done to simplify
        // generated bindings by default. Borrowed handles always use a borrow,
        // however.
        let first_mode = if info.owned
            || !info.borrowed
            || matches!(self.gen.opts.ownership, Ownership::Owning)
            || info.has_own_handle
        {
            if info.has_borrow_handle {
                TypeMode::HandlesBorrowed("'a")
            } else {
                TypeMode::Owned
            }
        } else {
            assert!(!self.uses_two_names(&info));
            TypeMode::AllBorrowed("'a")
        };
        result.push((self.result_name(ty), first_mode));
        if self.uses_two_names(&info) {
            result.push((self.param_name(ty), TypeMode::AllBorrowed("'a")));
        }
        result
    }

    fn print_typedef_record(
        &mut self,
        id: TypeId,
        record: &Record,
        docs: &Docs,
        derive_component: bool,
    ) {
        let info = self.info(id);
        // We use a BTree set to make sure we don't have any duplicates and we have a stable order
        let additional_derives: BTreeSet<String> = self
            .gen
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);

            if derive_component {
                self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
                if lt.is_none() {
                    self.push_str("#[derive(wasmtime::component::Lift)]\n");
                }
                self.push_str("#[derive(wasmtime::component::Lower)]\n");
                self.push_str("#[component(record)]\n");
            }
            let mut derives = additional_derives.clone();
            if info.is_copy() {
                self.push_str("#[repr(C)]\n");
                derives.extend(["Copy", "Clone"].into_iter().map(|s| s.to_string()));
            } else if info.is_clone() {
                derives.insert("Clone".to_string());
            }
            if !derives.is_empty() {
                self.push_str("#[derive(");
                self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
                self.push_str(")]\n")
            }
            self.push_str(&format!("pub struct {}", name));
            self.print_generics(lt);
            self.push_str(" {\n");
            for field in record.fields.iter() {
                self.rustdoc(&field.docs);
                if derive_component {
                    self.push_str(&format!("#[component(name = \"{}\")]\n", field.name));
                }
                self.push_str("pub ");
                self.push_str(&to_rust_ident(&field.name));
                self.push_str(": ");
                self.print_ty(&field.ty, mode);
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.push_str("impl");
            self.print_generics(lt);
            self.push_str(" ::core::fmt::Debug for ");
            self.push_str(&name);
            self.print_generics(lt);
            self.push_str(" {\n");
            self.push_str(
                "fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
            );
            self.push_str(&format!("f.debug_struct(\"{}\")", name));
            for field in record.fields.iter() {
                self.push_str(&format!(
                    ".field(\"{}\", &self.{})",
                    field.name,
                    to_rust_ident(&field.name)
                ));
            }
            self.push_str(".finish()\n");
            self.push_str("}\n");
            self.push_str("}\n");

            if info.error {
                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" ::core::fmt::Display for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {\n");
                self.push_str(
                    "fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
                );
                self.push_str("write!(f, \"{:?}\", self)\n");
                self.push_str("}\n");
                self.push_str("}\n");
                if self.gen.opts.std_feature {
                    self.push_str("#[cfg(feature = \"std\")]");
                }
                self.push_str("impl std::error::Error for ");
                self.push_str(&name);
                self.push_str("{}\n");
            }
        }
    }

    fn print_typedef_tuple(&mut self, id: TypeId, tuple: &Tuple, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(lt);
            self.push_str(" = (");
            for ty in tuple.types.iter() {
                self.print_ty(ty, mode);
                self.push_str(",");
            }
            self.push_str(");\n");
        }
    }

    fn print_typedef_variant(
        &mut self,
        id: TypeId,
        variant: &Variant,
        docs: &Docs,
        derive_component: bool,
    ) where
        Self: Sized,
    {
        self.print_rust_enum(
            id,
            variant.cases.iter().map(|c| {
                (
                    c.name.to_upper_camel_case(),
                    Some(c.name.clone()),
                    &c.docs,
                    c.ty.as_ref(),
                )
            }),
            docs,
            if derive_component {
                Some("variant")
            } else {
                None
            },
        );
    }

    fn print_rust_enum<'b>(
        &mut self,
        id: TypeId,
        cases: impl IntoIterator<Item = (String, Option<String>, &'b Docs, Option<&'b Type>)> + Clone,
        docs: &Docs,
        derive_component: Option<&str>,
    ) where
        Self: Sized,
    {
        let info = self.info(id);
        // We use a BTree set to make sure we don't have any duplicates and have a stable order
        let additional_derives: BTreeSet<String> = self
            .gen
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();
        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            if let Some(derive_component) = derive_component {
                self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
                if lt.is_none() {
                    self.push_str("#[derive(wasmtime::component::Lift)]\n");
                }
                self.push_str("#[derive(wasmtime::component::Lower)]\n");
                self.push_str(&format!("#[component({})]\n", derive_component));
            }
            let mut derives = additional_derives.clone();
            if info.is_copy() {
                derives.extend(["Copy", "Clone"].into_iter().map(|s| s.to_string()));
            } else if info.is_clone() {
                derives.insert("Clone".to_string());
            }
            if !derives.is_empty() {
                self.push_str("#[derive(");
                self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
                self.push_str(")]\n")
            }
            self.push_str(&format!("pub enum {name}"));
            self.print_generics(lt);
            self.push_str("{\n");
            for (case_name, component_name, docs, payload) in cases.clone() {
                self.rustdoc(docs);
                if derive_component.is_some() {
                    if let Some(n) = component_name {
                        self.push_str(&format!("#[component(name = \"{}\")] ", n));
                    }
                }
                self.push_str(&case_name);
                if let Some(ty) = payload {
                    self.push_str("(");
                    self.print_ty(ty, mode);
                    self.push_str(")")
                }
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.print_rust_enum_debug(
                id,
                mode,
                &name,
                cases
                    .clone()
                    .into_iter()
                    .map(|(name, _attr, _docs, ty)| (name, ty)),
            );

            if info.error {
                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" ::core::fmt::Display for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {\n");
                self.push_str(
                    "fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
                );
                self.push_str("write!(f, \"{:?}\", self)\n");
                self.push_str("}\n");
                self.push_str("}\n");
                self.push_str("\n");

                if self.gen.opts.std_feature {
                    self.push_str("#[cfg(feature = \"std\")]");
                }
                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" std::error::Error for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {}\n");
            }
        }
    }

    fn print_rust_enum_debug<'b>(
        &mut self,
        id: TypeId,
        mode: TypeMode,
        name: &str,
        cases: impl IntoIterator<Item = (String, Option<&'b Type>)>,
    ) where
        Self: Sized,
    {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        self.push_str("impl");
        self.print_generics(lt);
        self.push_str(" ::core::fmt::Debug for ");
        self.push_str(name);
        self.print_generics(lt);
        self.push_str(" {\n");
        self.push_str(
            "fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
        );
        self.push_str("match self {\n");
        for (case_name, payload) in cases {
            self.push_str(name);
            self.push_str("::");
            self.push_str(&case_name);
            if payload.is_some() {
                self.push_str("(e)");
            }
            self.push_str(" => {\n");
            self.push_str(&format!("f.debug_tuple(\"{}::{}\")", name, case_name));
            if payload.is_some() {
                self.push_str(".field(e)");
            }
            self.push_str(".finish()\n");
            self.push_str("}\n");
        }
        self.push_str("}\n");
        self.push_str("}\n");
        self.push_str("}\n");
    }

    fn print_typedef_option(&mut self, id: TypeId, payload: &Type, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(lt);
            self.push_str("= Option<");
            self.print_ty(payload, mode);
            self.push_str(">;\n");
        }
    }

    fn print_typedef_result(&mut self, id: TypeId, result: &Result_, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(lt);
            self.push_str("= Result<");
            self.print_optional_ty(result.ok.as_ref(), mode);
            self.push_str(",");
            self.print_optional_ty(result.err.as_ref(), mode);
            self.push_str(">;\n");
        }
    }

    fn print_typedef_enum(
        &mut self,
        id: TypeId,
        name: &str,
        enum_: &Enum,
        docs: &Docs,
        attrs: &[String],
        case_attr: Box<dyn Fn(&EnumCase) -> String>,
    ) where
        Self: Sized,
    {
        let info = self.info(id);

        let name = to_upper_camel_case(name);
        self.rustdoc(docs);
        for attr in attrs {
            self.push_str(&format!("{}\n", attr));
        }
        self.push_str("#[repr(");
        self.int_repr(enum_.tag());
        self.push_str(")]\n");
        // We use a BTree set to make sure we don't have any duplicates and a stable order
        let mut derives: BTreeSet<String> = self
            .gen
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();
        derives.extend(
            ["Clone", "Copy", "PartialEq", "Eq"]
                .into_iter()
                .map(|s| s.to_string()),
        );
        self.push_str("#[derive(");
        self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
        self.push_str(")]\n");
        self.push_str(&format!("pub enum {name} {{\n"));
        for case in enum_.cases.iter() {
            self.rustdoc(&case.docs);
            self.push_str(&case_attr(case));
            self.push_str(&case.name.to_upper_camel_case());
            self.push_str(",\n");
        }
        self.push_str("}\n");

        // Auto-synthesize an implementation of the standard `Error` trait for
        // error-looking types based on their name.
        if info.error {
            self.push_str("impl ");
            self.push_str(&name);
            self.push_str("{\n");

            self.push_str("pub fn name(&self) -> &'static str {\n");
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_upper_camel_case());
                self.push_str(" => \"");
                self.push_str(case.name.as_str());
                self.push_str("\",\n");
            }
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("pub fn message(&self) -> &'static str {\n");
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_upper_camel_case());
                self.push_str(" => \"");
                if let Some(contents) = &case.docs.contents {
                    self.push_str(contents.trim());
                }
                self.push_str("\",\n");
            }
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("}\n");

            self.push_str("impl ::core::fmt::Debug for ");
            self.push_str(&name);
            self.push_str(
                "{\nfn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
            );
            self.push_str("f.debug_struct(\"");
            self.push_str(&name);
            self.push_str("\")\n");
            self.push_str(".field(\"code\", &(*self as i32))\n");
            self.push_str(".field(\"name\", &self.name())\n");
            self.push_str(".field(\"message\", &self.message())\n");
            self.push_str(".finish()\n");
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("impl ::core::fmt::Display for ");
            self.push_str(&name);
            self.push_str(
                "{\nfn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {\n",
            );
            self.push_str("write!(f, \"{} (error {})\", self.name(), *self as i32)\n");
            self.push_str("}\n");
            self.push_str("}\n");
            self.push_str("\n");
            if self.gen.opts.std_feature {
                self.push_str("#[cfg(feature = \"std\")]");
            }
            self.push_str("impl std::error::Error for ");
            self.push_str(&name);
            self.push_str("{}\n");
        } else {
            self.print_rust_enum_debug(
                id,
                TypeMode::Owned,
                &name,
                enum_
                    .cases
                    .iter()
                    .map(|c| (c.name.to_upper_camel_case(), None)),
            )
        }
    }

    fn print_typedef_alias(&mut self, id: TypeId, ty: &Type, docs: &Docs) {
        if self.is_exported_resource(id) {
            let target = dealias(self.resolve, id);
            let ty = &self.resolve.types[target];
            // TODO: We could wait until we know how a resource (and its
            // aliases) is used prior to generating declarations.  For example,
            // if only borrows are used, no need to generate the `Own{name}`
            // version.
            self.mark_resource_owned(target);
            for prefix in ["Own", ""] {
                self.rustdoc(docs);
                self.push_str(&format!(
                    "pub type {prefix}{} = {};\n",
                    self.resolve.types[id]
                        .name
                        .as_deref()
                        .unwrap()
                        .to_upper_camel_case(),
                    self.type_path_with_name(
                        target,
                        format!(
                            "{prefix}{}",
                            ty.name.as_deref().unwrap().to_upper_camel_case()
                        )
                    )
                ));
            }
        } else {
            let info = self.info(id);
            for (name, mode) in self.modes_of(id) {
                self.rustdoc(docs);
                self.push_str(&format!("pub type {name}"));
                let lt = self.lifetime_for(&info, mode);
                self.print_generics(lt);
                self.push_str(" = ");
                self.print_ty(ty, mode);
                self.push_str(";\n");
            }
        }
    }

    fn print_type_list(&mut self, id: TypeId, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(lt);
            self.push_str(" = ");
            self.print_list(ty, mode);
            self.push_str(";\n");
        }
    }

    fn param_name(&self, ty: TypeId) -> String {
        let info = self.info(ty);
        let name = to_upper_camel_case(self.resolve.types[ty].name.as_ref().unwrap());
        if self.uses_two_names(&info) {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, ty: TypeId) -> String {
        let info = self.info(ty);
        let name = to_upper_camel_case(self.resolve.types[ty].name.as_ref().unwrap());
        if self.uses_two_names(&info) {
            format!("{}Result", name)
        } else if self.is_exported_resource(ty) {
            format!("Own{name}")
        } else {
            name
        }
    }

    fn uses_two_names(&self, info: &TypeInfo) -> bool {
        // Types are only duplicated if explicitly requested ...
        matches!(
            self.gen.opts.ownership,
            Ownership::Borrowing {
                duplicate_if_necessary: true
            }
        )
            // ... and if they're both used in a borrowed/owned context
            && info.borrowed
            && info.owned
            // ... and they have a list ...
            && info.has_list
            // ... and if there's NOT an `own` handle since those are always
            // done by ownership.
            && !info.has_own_handle
    }

    fn lifetime_for(&self, info: &TypeInfo, mode: TypeMode) -> Option<&'static str> {
        let lt = match mode {
            TypeMode::AllBorrowed(s) | TypeMode::HandlesBorrowed(s) => s,
            TypeMode::Owned => return None,
        };
        if info.has_borrow_handle {
            return Some(lt);
        }
        if matches!(self.gen.opts.ownership, Ownership::Owning) {
            return None;
        }
        // No lifetimes needed unless this has a list.
        if !info.has_list {
            return None;
        }
        // If two names are used then this type will have an owned and a
        // borrowed copy and the borrowed copy is being used, so it needs a
        // lifetime. Otherwise if it's only borrowed and not owned then this can
        // also use a lifetime since it's not needed in two contexts and only
        // the borrowed version of the structure was generated.
        if self.uses_two_names(info) || (info.borrowed && !info.owned) {
            Some(lt)
        } else {
            None
        }
    }

    // fn ownership(&self) -> Ownership {
    //     self.gen.opts.ownership
    // }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let InterfaceName { path, remapped } = &self.gen.interface_names[&interface];
        if *remapped {
            let mut path_to_root = self.path_to_root();
            path_to_root.push_str(path);
            return Some(path_to_root);
        } else {
            let mut full_path = String::new();
            if let Identifier::Interface(cur, name) = self.identifier {
                if cur == interface {
                    return None;
                }
                if !self.in_import {
                    full_path.push_str("super::");
                }
                match name {
                    WorldKey::Name(_) => {
                        full_path.push_str("super::");
                    }
                    WorldKey::Interface(_) => {
                        full_path.push_str("super::super::super::");
                    }
                }
            }
            full_path.push_str(&path);
            Some(full_path)
        }
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

    pub fn mark_resource_owned(&mut self, resource: TypeId) {
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
                    #[repr(transparent)]
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
            // NB: errors are ignored here since they'll generate an error
            // through the `generate_exports` method above.
            let impl_name = self
                .gen
                .lookup_export(&self.export_key(Some(name)))
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
                        unsafe fn new(_rep: usize) -> u32 {{
                            #[cfg(not(target_arch = "wasm32"))]
                            unreachable!();

                            #[cfg(target_arch = "wasm32")]
                            {{
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-new]{name}"]
                                    fn new(_: usize) -> u32;
                                }}
                                new(_rep)
                            }}
                        }}

                        unsafe fn rep(_handle: u32) -> usize {{
                            #[cfg(not(target_arch = "wasm32"))]
                            unreachable!();

                            #[cfg(target_arch = "wasm32")]
                            {{
                                #[link(wasm_import_module = "[export]{module}")]
                                extern "C" {{
                                    #[link_name = "[resource-rep]{name}"]
                                    fn rep(_: u32) -> usize;
                                }}
                                rep(_handle)
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
                     unsafe fn drop(_handle: u32) {{
                         #[cfg(not(target_arch = "wasm32"))]
                         unreachable!();

                         #[cfg(target_arch = "wasm32")]
                         {{
                             #[link(wasm_import_module = "{wasm_import_module}")]
                             extern "C" {{
                                 #[link_name = "[resource-drop]{name}"]
                                 fn drop(_: u32);
                             }}

                             drop(_handle);
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
