use heck::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_bindgen_core::{
    uwrite, uwriteln, wit_parser::*, Files, InterfaceGenerator as _, Source, TypeInfo, Types,
    WorldGenerator,
};
use wit_bindgen_gen_rust_lib::{to_rust_ident, FnSig, RustGenerator, TypeMode};

#[derive(Default)]
struct Wasmtime {
    src: Source,
    opts: Opts,
    imports: Vec<String>,
    exports: Exports,
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
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = Wasmtime::default();
        r.opts = self;
        Box::new(r)
    }
}

impl WorldGenerator for Wasmtime {
    fn import(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::Owned);
        gen.types();
        gen.generate_add_to_linker(name);

        let snake = name.to_snake_case();
        let module = &gen.src[..];

        uwriteln!(
            self.src,
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[allow(unused_imports)]
                    use wit_bindgen_host_wasmtime_rust::{{wasmtime, anyhow}};

                    {module}
                }}
            "
        );

        self.imports.push(snake); // TODO
    }

    fn export(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::AllBorrowed("'a"));
        gen.types();
        for func in iface.functions.iter() {
            gen.append_guest_export(Some(name), func);
        }

        let snake = name.to_snake_case();
        let module = &gen.src[..];

        uwriteln!(
            self.src,
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[allow(unused_imports)]
                    use wit_bindgen_host_wasmtime_rust::{{wasmtime, anyhow}};

                    {module}
                }}
            "
        );
    }

    fn export_default(&mut self, _name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::AllBorrowed("'a"));
        gen.types();
        for func in iface.functions.iter() {
            gen.append_guest_export(None, func);
        }

        let src = gen.src;
        self.src.push_str(&src);
    }

    fn finish(&mut self, name: &str, files: &mut Files) {
        let camel = name.to_upper_camel_case();
        uwriteln!(self.src, "pub struct {camel} {{");
        for (name, (ty, _)) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name}: {ty},");
        }
        self.src.push_str("}\n");

        uwriteln!(
            self.src,
            "
                impl {camel} {{
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    pub fn instantiate<T>(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        component: &wasmtime::component::Component,
                        linker: &wasmtime::component::Linker<T>,
                    ) -> anyhow::Result<(Self, wasmtime::component::Instance)> {{
                        let instance = linker.instantiate(&mut store, component)?;
                        Ok((Self::new(store, &instance)?, instance))
                    }}

                    /// Low-level creation wrapper for wrapping up the exports
                    /// of the `instance` provided in this structure of wasm
                    /// exports.
                    ///
                    /// This function will extract exports from the `instance`
                    /// defined within `store` and wrap them all up in the
                    /// returned structure which can be used to interact with
                    /// the wasm module.
                    pub fn new(
                        mut store: impl wasmtime::AsContextMut,
                        instance: &wasmtime::component::Instance,
                    ) -> anyhow::Result<Self> {{
                        let mut store = store.as_context_mut();
            ",
        );
        for (name, (_, get)) in self.exports.fields.iter() {
            uwriteln!(self.src, "let {name} = {get};");
        }
        uwriteln!(self.src, "Ok({camel} {{");
        for (name, _) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name},");
        }
        uwriteln!(self.src, "}})");
        uwriteln!(self.src, "}}");

        for func in self.exports.funcs.iter() {
            self.src.push_str(func);
        }

        uwriteln!(self.src, "}}");

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

        files.push(&format!("{name}.rs"), src.as_bytes());
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut Wasmtime,
    iface: &'a Interface,
    default_param_mode: TypeMode,
    types: Types,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(
        gen: &'a mut Wasmtime,
        iface: &'a Interface,
        default_param_mode: TypeMode,
    ) -> InterfaceGenerator<'a> {
        let mut types = Types::default();
        types.analyze(iface);
        InterfaceGenerator {
            src: Source::default(),
            gen,
            iface,
            types,
            default_param_mode,
        }
    }

    fn print_result_ty(&mut self, results: &Results, mode: TypeMode) {
        match results {
            Results::Named(rs) => match rs.len() {
                0 => self.push_str("()"),
                1 => self.print_ty(&rs[0].1, mode),
                _ => {
                    self.push_str("(");
                    for (i, (_, ty)) in rs.iter().enumerate() {
                        if i > 0 {
                            self.push_str(", ")
                        }
                        self.print_ty(ty, mode)
                    }
                    self.push_str(")");
                }
            },
            Results::Anon(ty) => self.print_ty(ty, mode),
        }
    }

    fn generate_add_to_linker(&mut self, name: &str) {
        let camel = name.to_upper_camel_case();

        // Generate the `pub trait` which represents the host functionality for
        // this import.
        uwriteln!(self.src, "pub trait {camel}: Sized {{");
        for func in self.iface.functions.iter() {
            let mut fnsig = FnSig::default();
            fnsig.private = true;
            fnsig.self_arg = Some("&mut self".to_string());

            // These trait method args used to be TypeMode::LeafBorrowed, but wasmtime
            // Lift is not impled for borrowed types, so I don't think we can
            // support that anymore?
            self.print_docs_and_params(func, TypeMode::Owned, &fnsig);
            self.push_str(" -> ");
            self.print_result_ty(&func.results, TypeMode::Owned);
        }
        uwriteln!(self.src, "}}");

        uwriteln!(
            self.src,
            "
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> anyhow::Result<()>
                    where U: {camel},
                {{
            "
        );
        uwriteln!(self.src, "let mut inst = linker.instance(\"{name}\")?;");
        for func in self.iface.functions.iter() {
            uwrite!(self.src, "inst.func_wrap(\"{}\", ", func.name);
            self.generate_guest_import_closure(func);
            uwriteln!(self.src, ")?;")
        }
        uwriteln!(self.src, "Ok(())");
        uwriteln!(self.src, "}}");
    }

    fn generate_guest_import_closure(&mut self, func: &Function) {
        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        self.src
            .push_str("move |mut caller: wasmtime::StoreContextMut<'_, T>");
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, ", arg{} :", i);
            // Lift is required to be impled for this type, so we can't use
            // a borrowed type:
            self.print_ty(&param.1, TypeMode::Owned);
        }
        self.src.push_str("| {\n");

        if self.gen.opts.tracing {
            self.src.push_str(&format!(
                "
                   let span = wit_bindgen_host_wasmtime_rust::tracing::span!(
                       wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,
                       \"wit-bindgen guest import\",
                       module = \"{}\",
                       function = \"{}\",
                   );
                   let _enter = span.enter();
               ",
                self.iface.name, func.name,
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
    }

    fn append_guest_export(&mut self, ns: Option<&str>, func: &Function) {
        let prev = mem::take(&mut self.src);
        uwrite!(
            self.src,
            "pub fn {}(&self, mut store: impl wasmtime::AsContextMut, ",
            func.name.to_snake_case(),
        );
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(&param.1, TypeMode::AllBorrowed("'_"));
            self.push_str(",");
        }
        self.src.push_str(") -> anyhow::Result<");
        self.print_result_ty(&func.results, TypeMode::Owned);
        self.src.push_str("> {\n");

        if self.gen.opts.tracing {
            self.src.push_str(&format!(
                "
                       let span = wit_bindgen_host_wasmtime_rust::tracing::span!(
                           wit_bindgen_host_wasmtime_rust::tracing::Level::TRACE,
                           \"wit-bindgen guest export\",
                           module = \"{}\",
                           function = \"{}\",
                       );
                       let _enter = span.enter();
                   ",
                ns.unwrap_or("default"),
                func.name,
            ));
        }

        self.src.push_str("let callee = unsafe {\n");
        self.src.push_str("wasmtime::component::TypedFunc::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, TypeMode::AllBorrowed("'_"));
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }
        uwriteln!(
            self.src,
            ")>::new_unchecked(self.{})",
            func.name.to_snake_case()
        );
        self.src.push_str("};\n");
        self.src.push_str("let (");
        for (i, _) in func.results.iter_types().enumerate() {
            uwrite!(self.src, "ret{},", i);
        }
        uwrite!(self.src, ") = callee.call(store.as_context_mut(), (");
        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}, ", i);
        }
        uwriteln!(self.src, "))?;");

        uwriteln!(self.src, "callee.post_return(store.as_context_mut())?;");

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

        self.src.push_str("*instance.get_typed_func::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, TypeMode::AllBorrowed("'_"));
            self.push_str(", ");
        }

        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }

        self.src.push_str("), _>(&mut store, \"");
        self.src.push_str(&func.name);
        self.src.push_str("\")?.func()");
        let getter: String = mem::replace(&mut self.src, prev).into();

        self.gen.exports.funcs.push(pub_func);
        let prev = self.gen.exports.fields.insert(
            to_rust_ident(&func.name),
            ("wasmtime::component::Func".to_string(), getter),
        );
        assert!(prev.is_none());
    }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn default_param_mode(&self) -> TypeMode {
        self.default_param_mode
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

    fn print_borrowed_slice(&mut self, mutbl: bool, ty: &Type, lifetime: &'static str) {
        self.print_rust_slice(mutbl, ty, lifetime);
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

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &Record, docs: &Docs) {
        self.print_typedef_record(id, record, docs, true);
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, tuple: &Tuple, docs: &Docs) {
        self.print_typedef_tuple(id, tuple, docs);
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.rustdoc(docs);
        self.src.push_str("wasmtime::component::flags!(\n");
        self.src
            .push_str(&format!("{} {{\n", name.to_upper_camel_case()));
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

    fn type_variant(&mut self, id: TypeId, _name: &str, variant: &Variant, docs: &Docs) {
        self.print_typedef_variant(id, variant, docs, true);
    }

    fn type_union(&mut self, id: TypeId, _name: &str, union: &Union, docs: &Docs) {
        self.print_typedef_union(id, union, docs, true);
    }

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        self.print_typedef_option(id, payload, docs);
    }

    fn type_result(&mut self, id: TypeId, _name: &str, result: &Result_, docs: &Docs) {
        self.print_typedef_result(id, result, docs);
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        self.print_typedef_enum(id, name, enum_, docs,
           &["#[derive(wasmtime::component::ComponentType, wasmtime::component::Lift, wasmtime::component::Lower)]".to_owned(),
           "#[component(enum)]".to_owned()],
           Box::new(|case| format!("#[component(name = \"{}\")]", case.name))
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
