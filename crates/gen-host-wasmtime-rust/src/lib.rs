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
use wit_bindgen_gen_rust_lib::{FnSig, RustGenerator, TypeMode};
use wit_component::ComponentInterfaces;

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

    /// Whether or not to use async rust functions and traits.
    #[cfg_attr(feature = "clap", arg(long = "async"))]
    pub async_: bool,
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
        gen.generate_from_error_impls();
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
        gen.generate_from_error_impls();

        let camel = name.to_upper_camel_case();
        uwriteln!(gen.src, "pub struct {camel} {{");
        for func in iface.functions.iter() {
            uwriteln!(
                gen.src,
                "{}: wasmtime::component::Func,",
                func.name.to_snake_case()
            );
        }
        uwriteln!(gen.src, "}}");

        uwriteln!(gen.src, "impl {camel} {{");
        uwrite!(
            gen.src,
            "
                pub fn new(
                    exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                ) -> anyhow::Result<{camel}> {{
            "
        );
        let fields = gen.extract_typed_functions();
        for (name, getter) in fields.iter() {
            uwriteln!(gen.src, "let {name} = {getter};");
        }
        uwriteln!(gen.src, "Ok({camel} {{");
        for (name, _) in fields.iter() {
            uwriteln!(gen.src, "{name},");
        }
        uwriteln!(gen.src, "}})");
        uwriteln!(gen.src, "}}");
        for func in iface.functions.iter() {
            gen.define_rust_guest_export(Some(name), func);
        }
        uwriteln!(gen.src, "}}");

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

        let getter = format!(
            "\
                {snake}::{camel}::new(
                    &mut exports.instance(\"{name}\")
                        .ok_or_else(|| anyhow::anyhow!(\"exported instance `{name}` not present\"))?
                )?\
            "
        );
        let prev = self
            .exports
            .fields
            .insert(snake.clone(), (format!("{snake}::{camel}"), getter));
        assert!(prev.is_none());
        self.exports.funcs.push(format!(
            "
                pub fn {snake}(&self) -> &{snake}::{camel} {{
                    &self.{snake}
                }}
            "
        ));
    }

    fn export_default(&mut self, _name: &str, iface: &Interface, _files: &mut Files) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::AllBorrowed("'a"));
        gen.types();
        let fields = gen.extract_typed_functions();
        for (name, getter) in fields {
            let prev = gen
                .gen
                .exports
                .fields
                .insert(name, ("wasmtime::component::Func".to_string(), getter));
            assert!(prev.is_none());
        }

        for func in iface.functions.iter() {
            let prev = mem::take(&mut gen.src);
            gen.define_rust_guest_export(None, func);
            let func = mem::replace(&mut gen.src, prev);
            gen.gen.exports.funcs.push(func.to_string());
        }

        let src = gen.src;
        self.src.push_str(&src);
    }

    fn finish(&mut self, name: &str, _interfaces: &ComponentInterfaces, files: &mut Files) {
        let camel = name.to_upper_camel_case();
        uwriteln!(self.src, "pub struct {camel} {{");
        for (name, (ty, _)) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name}: {ty},");
        }
        self.src.push_str("}\n");

        let (async_, async__, send, await_) = if self.opts.async_ {
            ("async", "_async", ":Send", ".await")
        } else {
            ("", "", "", "")
        };

        uwriteln!(
            self.src,
            "
                impl {camel} {{
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    pub {async_} fn instantiate{async__}<T {send}>(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        component: &wasmtime::component::Component,
                        linker: &wasmtime::component::Linker<T>,
                    ) -> anyhow::Result<(Self, wasmtime::component::Instance)> {{
                        let instance = linker.instantiate{async__}(&mut store, component){await_}?;
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
                        let mut exports = instance.exports(&mut store);
                        let mut exports = exports.root();
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

    fn special_case_host_error(&self, results: &Results) -> Option<&Result_> {
        // We only support the wit_bindgen_host_wasmtime_rust::Error case when
        // a function has just one result, which is itself a `result<a, e>`, and the
        // `e` is *not* a primitive (i.e. defined in std) type.
        let mut i = results.iter_types();
        if i.len() == 1 {
            match i.next().unwrap() {
                Type::Id(id) => match &self.iface.types[*id].kind {
                    TypeDefKind::Result(r) => match r.err {
                        Some(Type::Id(_)) => Some(&r),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }

    fn generate_add_to_linker(&mut self, name: &str) {
        let camel = name.to_upper_camel_case();

        if self.gen.opts.async_ {
            uwriteln!(self.src, "#[wit_bindgen_host_wasmtime_rust::async_trait]")
        }
        // Generate the `pub trait` which represents the host functionality for
        // this import.
        uwriteln!(self.src, "pub trait {camel}: Sized {{");
        for func in self.iface.functions.iter() {
            let mut fnsig = FnSig::default();
            fnsig.async_ = self.gen.opts.async_;
            fnsig.private = true;
            fnsig.self_arg = Some("&mut self".to_string());

            self.print_docs_and_params(func, TypeMode::Owned, &fnsig);
            self.push_str(" -> ");

            if let Some(r) = self.special_case_host_error(&func.results).cloned() {
                // Functions which have a single result `result<ok,err>` get special
                // cased to use the host_wasmtime_rust::Error<err>, making it possible
                // for them to trap or use `?` to propogate their errors
                self.push_str("wit_bindgen_host_wasmtime_rust::Result<");
                if let Some(ok) = r.ok {
                    self.print_ty(&ok, TypeMode::Owned);
                } else {
                    self.push_str("()");
                }
                self.push_str(",");
                if let Some(err) = r.err {
                    self.print_ty(&err, TypeMode::Owned);
                } else {
                    self.push_str("()");
                }
                self.push_str(">");
            } else {
                // All other functions get their return values wrapped in an anyhow::Result.
                // Returning the anyhow::Error case can be used to trap.
                self.push_str("anyhow::Result<");
                self.print_result_ty(&func.results, TypeMode::Owned);
                self.push_str(">");
            }

            self.push_str(";\n");
        }
        uwriteln!(self.src, "}}");

        let where_clause = if self.gen.opts.async_ {
            format!("T: Send, U: {camel} + Send")
        } else {
            format!("U: {camel}")
        };
        uwriteln!(
            self.src,
            "
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> anyhow::Result<()>
                    where {where_clause},
                {{
            "
        );
        uwriteln!(self.src, "let mut inst = linker.instance(\"{name}\")?;");
        for func in self.iface.functions.iter() {
            uwrite!(
                self.src,
                "inst.{}(\"{}\", ",
                if self.gen.opts.async_ {
                    "func_wrap_async"
                } else {
                    "func_wrap"
                },
                func.name
            );
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
            .push_str("move |mut caller: wasmtime::StoreContextMut<'_, T>, (");
        for (i, _param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }
        self.src.push_str(") : (");
        for param in func.params.iter() {
            // Lift is required to be impled for this type, so we can't use
            // a borrowed type:
            self.print_ty(&param.1, TypeMode::Owned);
            self.src.push_str(", ");
        }
        self.src.push_str(") |");
        if self.gen.opts.async_ {
            self.src.push_str(" Box::new(async move { \n");
        } else {
            self.src.push_str(" { \n");
        }

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
        if self.gen.opts.async_ {
            uwrite!(self.src, ").await;\n");
        } else {
            uwrite!(self.src, ");\n");
        }

        if self.special_case_host_error(&func.results).is_some() {
            uwrite!(
                self.src,
                "match r {{
                    Ok(a) => Ok((Ok(a),)),
                    Err(e) => match e.downcast() {{
                        Ok(api_error) => Ok((Err(api_error),)),
                        Err(anyhow_error) => Err(anyhow_error),
                    }}
                }}"
            );
        } else if func.results.iter_types().len() == 1 {
            uwrite!(self.src, "Ok((r?,))\n");
        } else {
            uwrite!(self.src, "r\n");
        }

        if self.gen.opts.async_ {
            // Need to close Box::new and async block
            self.src.push_str("})");
        } else {
            self.src.push_str("}");
        }
    }

    fn extract_typed_functions(&mut self) -> Vec<(String, String)> {
        let prev = mem::take(&mut self.src);
        let mut ret = Vec::new();
        for func in self.iface.functions.iter() {
            let snake = func.name.to_snake_case();
            uwrite!(self.src, "*exports.typed_func::<(");
            for (_, ty) in func.params.iter() {
                self.print_ty(ty, TypeMode::AllBorrowed("'_"));
                self.push_str(", ");
            }
            self.src.push_str("), (");
            for ty in func.results.iter_types() {
                self.print_ty(ty, TypeMode::Owned);
                self.push_str(", ");
            }
            self.src.push_str(")>(\"");
            self.src.push_str(&func.name);
            self.src.push_str("\")?.func()");

            ret.push((snake, mem::take(&mut self.src).to_string()));
        }
        self.src = prev;
        return ret;
    }

    fn define_rust_guest_export(&mut self, ns: Option<&str>, func: &Function) {
        let (async_, async__, await_) = if self.gen.opts.async_ {
            ("async", "_async", ".await")
        } else {
            ("", "", "")
        };

        self.rustdoc(&func.docs);
        uwrite!(
            self.src,
            "pub {async_} fn {}<S: wasmtime::AsContextMut>(&self, mut store: S, ",
            func.name.to_snake_case(),
        );
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(&param.1, TypeMode::AllBorrowed("'_"));
            self.push_str(",");
        }
        self.src.push_str(") -> anyhow::Result<");
        self.print_result_ty(&func.results, TypeMode::Owned);

        if self.gen.opts.async_ {
            self.src
                .push_str("> where <S as wasmtime::AsContext>::Data: Send {\n");
        } else {
            self.src.push_str("> {\n");
        }

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
        uwrite!(
            self.src,
            ") = callee.call{async__}(store.as_context_mut(), ("
        );
        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}, ", i);
        }
        uwriteln!(self.src, ")){await_}?;");

        uwriteln!(
            self.src,
            "callee.post_return{async__}(store.as_context_mut()){await_}?;"
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
        self.src.push_str(")\n");

        // End function body
        self.src.push_str("}\n");
    }

    fn generate_from_error_impls(&mut self) {
        for (id, ty) in self.iface.types.iter() {
            if ty.name.is_none() {
                continue;
            }
            let info = self.info(id);
            if info.error {
                for (name, mode) in self.modes_of(id) {
                    let name = name.to_upper_camel_case();
                    if self.lifetime_for(&info, mode).is_some() {
                        continue;
                    }
                    self.push_str("impl From<");
                    self.push_str(&name);
                    self.push_str("> for wit_bindgen_host_wasmtime_rust::Error<");
                    self.push_str(&name);
                    self.push_str("> {\n");
                    self.push_str("fn from(e: ");
                    self.push_str(&name);
                    self.push_str(") -> wit_bindgen_host_wasmtime_rust::Error::< ");
                    self.push_str(&name);
                    self.push_str("> {\n");
                    self.push_str("wit_bindgen_host_wasmtime_rust::Error::new(e)\n");
                    self.push_str("}\n");
                    self.push_str("}\n");
                }
            }
        }
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
