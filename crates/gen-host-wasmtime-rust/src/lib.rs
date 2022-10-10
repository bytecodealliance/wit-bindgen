use heck::*;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_bindgen_core::wit_parser::abi::AbiVariant;
use wit_bindgen_core::{
    uwrite, wit_parser::*, Direction, Files, Generator, Source, TypeInfo, Types,
};
use wit_bindgen_gen_rust_lib::{to_rust_ident, FnSig, RustGenerator, TypeMode};

#[derive(Default)]
pub struct Wasmtime {
    src: Source,
    opts: Opts,
    types: Types,
    guest_imports: HashMap<String, Vec<Import>>,
    guest_exports: HashMap<String, Exports>,
    in_import: bool,
    in_trait: bool,
    trait_name: String,
    sizes: SizeAlign,
}

struct Import {
    name: String,
    trait_signature: String,
    closure: String,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, (String, String)>,
    funcs: Vec<String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rustfmt: bool,

    /// Whether or not to emit `tracing` macro calls on function entry/exit.
    #[cfg_attr(feature = "clap", arg(long))]
    pub tracing: bool,

    /// A flag to indicate that all trait methods in imports should return a
    /// custom trait-defined error. Applicable for import bindings.
    #[cfg_attr(feature = "clap", arg(long))]
    pub custom_error: bool,
}

impl Opts {
    pub fn build(self) -> Wasmtime {
        let mut r = Wasmtime::new();
        r.opts = self;
        r
    }
}

impl Wasmtime {
    pub fn new() -> Wasmtime {
        Wasmtime::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses a reversed mapping! In the Wasmtime host-side
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

    fn print_result_ty(&mut self, iface: &Interface, results: &Results, mode: TypeMode) {
        match results {
            Results::Named(rs) => match rs.len() {
                0 => self.push_str("()"),
                1 => self.print_ty(iface, &rs[0].1, mode),
                _ => {
                    self.push_str("(");
                    for (i, (_, ty)) in rs.iter().enumerate() {
                        if i > 0 {
                            self.push_str(", ")
                        }
                        self.print_ty(iface, ty, mode)
                    }
                    self.push_str(")");
                }
            },
            Results::Anon(ty) => self.print_ty(iface, ty, mode),
        }
    }
}

impl RustGenerator for Wasmtime {
    fn default_param_mode(&self) -> TypeMode {
        if self.in_import {
            // The default here is that only leaf values can be borrowed because
            // otherwise lists and such need to be copied into our own memory.
            TypeMode::LeafBorrowed("'a")
        } else {
            // When we're calling wasm exports, however, there's no need to take
            // any ownership of anything from the host so everything is borrowed
            // in the parameter position.
            TypeMode::AllBorrowed("'a")
        }
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.types.get(ty)
    }

    fn types_mut(&mut self) -> &mut Types {
        &mut self.types
    }

    fn print_borrowed_slice(
        &mut self,
        iface: &Interface,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
    ) {
        self.print_rust_slice(iface, mutbl, ty, lifetime);
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str(" str");
    }
}

impl Generator for Wasmtime {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        let variant = Self::abi_variant(dir);
        self.types.analyze(iface);
        self.in_import = variant == AbiVariant::GuestImport;
        self.trait_name = iface.name.to_upper_camel_case();
        self.src.push_str(&format!(
            "#[allow(clippy::all)]\npub mod {} {{\n",
            iface.name.to_snake_case(),
        ));
        self.src.push_str(
            "#[allow(unused_imports)]\nuse wit_bindgen_host_wasmtime_rust::{wasmtime, anyhow};\n",
        );
        self.sizes.fill(iface);
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        self.src
            .push_str("#[derive(wasmtime::component::ComponentType, wasmtime::component::Lift, wasmtime::component::Lower)]\n");
        self.src.push_str("#[component(record)]\n");
        self.print_typedef_record(iface, id, record, docs);
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        tuple: &Tuple,
        docs: &Docs,
    ) {
        self.print_typedef_tuple(iface, id, tuple, docs);
    }

    fn type_flags(
        &mut self,
        _iface: &Interface,
        _id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        self.rustdoc(docs);
        self.src.push_str("wasmtime::component::flags!(\n");
        self.src.push_str(&format!("{} {{\n", name.to_camel_case()));
        for flag in flags.flags.iter() {
            // TODO wasmtime-component-macro doesnt support docs for flags rn
            uwrite!(
                self.src,
                "#[component(name=\"{}\")] const {};\n",
                flag.name,
                flag.name.to_shouty_snake_case()
            );
        }
        self.src.push_str("}\n");
        self.src.push_str(");\n\n");
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        self.print_typedef_variant(iface, id, variant, docs);
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        self.print_typedef_union(iface, id, union, docs);
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        self.print_typedef_option(iface, id, payload, docs);
    }

    fn type_result(
        &mut self,
        iface: &Interface,
        id: TypeId,
        _name: &str,
        result: &Result_,
        docs: &Docs,
    ) {
        self.print_typedef_result(iface, id, result, docs);
    }

    fn type_enum(&mut self, _iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs);
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(iface, id, ty, docs);
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_type_list(iface, id, ty, docs);
    }

    fn type_builtin(&mut self, iface: &Interface, _id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.to_upper_camel_case()));
        self.src.push_str(" = ");
        self.print_ty(iface, ty, TypeMode::Owned);
        self.src.push_str(";\n");
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "export" uses the "guest import" ABI variant on the inside of
    // this `Generator` implementation.
    fn export(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);

        // Generate the signature this function will have in the final trait
        let self_arg = "&mut self".to_string();
        self.in_trait = true;

        let mut fnsig = FnSig::default();
        fnsig.private = true;
        fnsig.self_arg = Some(self_arg);
        self.print_docs_and_params(iface, func, TypeMode::LeafBorrowed("'_"), &fnsig);
        self.push_str(" -> ");
        self.print_result_ty(iface, &func.results, TypeMode::Owned);
        self.in_trait = false;
        let trait_signature = mem::take(&mut self.src).into();

        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        self.src
            .push_str("move |mut caller: wasmtime::StoreContextMut<'_, T>");
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, ", arg{} :", i);
            self.print_ty(iface, &param.1, TypeMode::Owned);
        }
        self.src.push_str("| {\n");

        if self.opts.tracing {
            self.src.push_str(&format!(
                "
                    let span = wit_bindgen_host_wasmtime_rust::tracing::span!(
                        wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,
                        \"wit-bindgen abi\",
                        module = \"{}\",
                        function = \"{}\",
                    );
                    let _enter = span.enter();
                ",
                iface.name, func.name,
            ));
        }

        self.src.push_str("let host = get(caller.data_mut());\n");

        uwrite!(self.src, "let r = host.{}(", func.name.to_snake_case());
        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }
        uwrite!(self.src, ");\n");
        if func.results.iter_types().len() == 1 {
            uwrite!(self.src, "Ok((r,))\n");
        } else {
            uwrite!(self.src, "Ok(r)\n");
        }

        self.src.push_str("}");
        let closure = mem::replace(&mut self.src, prev).into();

        self.guest_imports
            .entry(iface.name.to_string())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.to_string(),
                closure,
                trait_signature,
            });
    }

    // As with `abi_variant` above, we're generating host-side bindings here
    // so a user "import" uses the "export" ABI variant on the inside of
    // this `Generator` implementation.
    fn import(&mut self, iface: &Interface, func: &Function) {
        let prev = mem::take(&mut self.src);
        uwrite!(
            self.src,
            "pub fn {}(&self, mut store: impl wasmtime::AsContextMut<Data = T>, ",
            func.name.to_snake_case(),
        );
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(iface, &param.1, TypeMode::Owned);
            self.push_str(",");
        }
        self.src.push_str(") -> anyhow::Result<");
        self.print_result_ty(iface, &func.results, TypeMode::Owned);
        self.src.push_str("> {\n");

        self.src.push_str("let (");
        for (i, _) in func.results.iter_types().enumerate() {
            uwrite!(self.src, "ret{},", i);
        }
        uwrite!(
            self.src,
            ") = self.{}.call(store.as_context_mut(), (",
            func.name.to_snake_case()
        );
        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}, ", i);
        }

        uwrite!(self.src, "))?;\n");

        uwrite!(
            self.src,
            "self.{}.post_return(store.as_context_mut())?;\n",
            func.name.to_snake_case()
        );

        self.src.push_str("Ok(");
        if func.results.iter_types().len() == 1 {
            self.src.push_str("ret0");
        } else {
            self.src.push_str("(");
            for (i, _) in func.results.iter_types().enumerate() {
                uwrite!(self.src, "ret{},", i);
            }
            self.src.push_str(")");
        }
        self.src.push_str(")");

        // End function body
        self.src.push_str("}\n");

        let pub_func = mem::replace(&mut self.src, prev).into();
        let prev = mem::take(&mut self.src);

        self.src.push_str("wasmtime::component::TypedFunc<(");
        // ComponentNamedList means using tuple for all:
        for (_, ty) in func.params.iter() {
            self.print_ty(iface, ty, TypeMode::Owned);
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(iface, ty, TypeMode::Owned);
            self.push_str(", ");
        }
        self.src.push_str(")>");

        let type_sig: String = mem::replace(&mut self.src, prev).into();
        let prev = mem::take(&mut self.src);

        self.src.push_str("instance.get_typed_func::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(iface, ty, TypeMode::Owned);
            self.push_str(", ");
        }

        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(iface, ty, TypeMode::Owned);
            self.push_str(", ");
        }

        self.src.push_str("), _>(&mut store, \"");
        self.src.push_str(&func.name);
        self.src.push_str("\")?");
        let getter: String = mem::replace(&mut self.src, prev).into();

        let exports = self
            .guest_exports
            .entry(iface.name.to_string())
            .or_insert_with(Exports::default);
        exports.funcs.push(pub_func);
        exports
            .fields
            .insert(to_rust_ident(&func.name), (type_sig, getter));
    }

    fn finish_one(&mut self, _iface: &Interface, files: &mut Files) {
        for (module, funcs) in sorted_iter(&self.guest_imports) {
            let module_camel = module.to_upper_camel_case();
            self.src.push_str("pub trait ");
            self.src.push_str(&module_camel);
            self.src.push_str(": Sized ");
            self.src.push_str("{\n");
            for f in funcs {
                self.src.push_str(&f.trait_signature);
                self.src.push_str(";\n\n");
            }
            self.src.push_str("}\n");
        }

        for (module, funcs) in mem::take(&mut self.guest_imports) {
            let module_camel = module.to_upper_camel_case();
            self.push_str(
                "\npub fn add_to_linker<T, U>(linker: &mut wasmtime::component::Linker<T>",
            );
            self.push_str(", get: impl Fn(&mut T) -> ");
            self.push_str("&mut U");
            self.push_str("+ Send + Sync + Copy + 'static) -> anyhow::Result<()> \n");
            self.push_str("where U: ");
            self.push_str(&module_camel);
            self.push_str("\n{\n");
            self.push_str(&format!("let mut inst = linker.instance(\"{}\")?;", module,));
            for f in funcs {
                self.push_str(&format!(
                    "inst.func_wrap(\"{}\", {})?;\n",
                    f.name, f.closure,
                ));
            }
            self.push_str("Ok(())\n}\n");
        }

        for (module, exports) in sorted_iter(&mem::take(&mut self.guest_exports)) {
            let name = module.to_upper_camel_case();

            uwrite!(self.src, "pub struct {}<T> {{\n", name);
            self.push_str("_phantom: std::marker::PhantomData<T>,");
            for (name, (ty, _)) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(": ");
                self.push_str(ty);
                self.push_str(",\n");
            }
            self.push_str("}\n");
            uwrite!(self.src, "impl<T> {}<T> {{\n", name);

            self.push_str(&format!(
                "
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    ///
                    /// The `linker` provided will have intrinsics added to it
                    /// automatically, so it's not necessary to call
                    /// `add_to_linker` beforehand. This function will
                    /// instantiate the `module` otherwise using `linker`, and
                    /// both an instance of this structure and the underlying
                    /// `wasmtime::Instance` will be returned.
                    pub fn instantiate(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        component: &wasmtime::component::Component,
                        linker: &mut wasmtime::component::Linker<T>,
                    ) -> anyhow::Result<(Self, wasmtime::component::Instance)> {{
                        let instance = linker.instantiate(&mut store, component)?;
                        Ok((Self::new(store, &instance)?, instance))
                    }}
                ",
            ));

            self.push_str(&format!(
                "
                    /// Low-level creation wrapper for wrapping up the exports
                    /// of the `instance` provided in this structure of wasm
                    /// exports.
                    ///
                    /// This function will extract exports from the `instance`
                    /// defined within `store` and wrap them all up in the
                    /// returned structure which can be used to interact with
                    /// the wasm module.
                    pub fn new(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        instance: &wasmtime::component::Instance,
                    ) -> anyhow::Result<Self> {{
                ",
            ));
            self.push_str("let mut store = store.as_context_mut();\n");
            for (name, (_, get)) in exports.fields.iter() {
                self.push_str("let ");
                self.push_str(&name);
                self.push_str("= ");
                self.push_str(&get);
                self.push_str(";\n");
            }
            self.push_str("Ok(");
            self.push_str(&name);
            self.push_str("{\n");
            self.push_str("_phantom: std::marker::PhantomData,");
            for (name, _) in exports.fields.iter() {
                self.push_str(name);
                self.push_str(",\n");
            }
            self.push_str("\n})\n");
            self.push_str("}\n");

            for func in exports.funcs.iter() {
                self.push_str(func);
            }

            self.push_str("}\n");
        }

        // Close the opening `mod`.
        self.push_str("}\n");

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

        files.push("bindings.rs", src.as_bytes());
    }
}

fn sorted_iter<K: Ord, V>(map: &HashMap<K, V>) -> impl Iterator<Item = (&K, &V)> {
    let mut list = map.into_iter().collect::<Vec<_>>();
    list.sort_by_key(|p| p.0);
    list.into_iter()
}
