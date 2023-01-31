
use std::fmt::Write;
use std::{collections::BTreeSet, mem};

use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::uwrite;

use wit_bindgen_core::{
    uwriteln,
    wit_parser::{
        Field, Flags, FlagsRepr, Function, Int, Interface, SizeAlign, Type, TypeDefKind, TypeId,
        World,
    },
    Files, InterfaceGenerator as _, Source, WorldGenerator,
};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        // ––––---- debugging purpose ----------
        // if cfg!(debug_assertions) {
        //     println!("process id: {}", std::process::id());
        //     std::thread::sleep(std::time::Duration::from_secs(5));
        // }
        // ––––---------------------------------

        Box::new(TinyGo {
            opts: self.clone(),
            ..TinyGo::default()
        })
    }
}

#[derive(Default)]
pub struct TinyGo {
    opts: Opts,
    src: Source,
    export_funcs: Vec<(String, String)>,
    world: String,
    needs_result_option: bool,
    needs_import_unsafe: bool,
    needs_fmt_import: bool,
}

impl TinyGo {
    fn interface<'a>(&'a mut self, iface: &'a Interface, name: &'a str) -> InterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        InterfaceGenerator {
            src: Source::default(),
            gen: self,
            iface,
            name,
            export_funcs: Vec::new(),
            public_anonymous_types: BTreeSet::new(),
        }
    }

    fn clean_up_export_funcs(&mut self) {
        self.export_funcs = vec![];
    }
}

impl WorldGenerator for TinyGo {
    fn preprocess(&mut self, name: &str) {
        self.world = name.to_string();
    }

    fn import(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// {name}\n"));

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.import(iface, func);
        }
        gen.finish();

        let src = mem::take(&mut gen.src);

        self.src.push_str(&src);
    }

    fn export(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// {name}\n"));
        self.clean_up_export_funcs();

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);

        if !self.export_funcs.is_empty() {
            let interface_var_name = &name.to_snake_case();
            let interface_name = &name.to_upper_camel_case();

            self.src
                .push_str(format!("var {interface_var_name} {interface_name} = nil\n").as_str());
            self.src.push_str(
                format!(
                    "func Set{interface_name}(i {interface_name}) {{\n    {interface_var_name} = i\n}}\n"
                )
                .as_str(),
            );
            self.src
                .push_str(format!("type {interface_name} interface {{\n").as_str());
            for (interface_func_declaration, _) in &self.export_funcs {
                self.src
                    .push_str(format!("{interface_func_declaration}\n").as_str());
            }
            self.src.push_str("}\n");

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(export_func);
            }
        }
    }

    fn export_default(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// default {name}\n"));
        self.clean_up_export_funcs();

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);

        if !self.export_funcs.is_empty() {
            let interface_var_name = &name.to_snake_case();
            let interface_name = &name.to_upper_camel_case();

            self.src
                .push_str(format!("var {interface_var_name} {interface_name} = nil\n").as_str());
            self.src.push_str(
                format!(
                    "func Set{interface_name}(i {interface_name}) {{\n    {interface_var_name} = i\n}}\n"
                )
                .as_str(),
            );
            self.src
                .push_str(format!("type {interface_name} interface {{\n").as_str());
            for (interface_func_declaration, _) in &self.export_funcs {
                self.src
                    .push_str(format!("{interface_func_declaration}\n").as_str());
            }
            self.src.push_str("}\n");

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(export_func);
            }
        }
    }

    fn finish(&mut self, world: &World, files: &mut Files) {
        let _world_name = self.world.clone();
        let mut header = Source::default();
        // add package
        header.push_str("package ");
        header.push_str(self.world.to_snake_case().as_str());
        header.push_str("\n\n");

        // import C
        header.push_str("// #include \"");
        header.push_str(self.world.to_snake_case().as_str());
        header.push_str(".h\"\n");
        header.push_str("import \"C\"\n\n");

        if self.needs_import_unsafe {
            header.push_str("import \"unsafe\"\n\n");
        }
        if self.needs_fmt_import {
            header.push_str("import \"fmt\"\n\n");
        }
        let header = mem::take(&mut header);
        let src = mem::take(&mut self.src);
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            header.as_bytes(),
        );
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            src.as_bytes(),
        );
        if self.needs_result_option {
            let mut result_option_src = Source::default();
            uwriteln!(
                result_option_src,
                "package variants

                // inspired from https://github.com/moznion/go-optional
                
                type optionKind int
                
                const (
                    none optionKind = iota
                    some
                )
                
                type Option[T any] struct {{
                    kind optionKind
                    val  T
                }}
                
                // IsNone returns true if the option is None.
                func (o Option[T]) IsNone() bool {{
                    return o.kind == none
                }}
                
                // IsSome returns true if the option is Some.
                func (o Option[T]) IsSome() bool {{
                    return o.kind == some
                }}
                
                // Unwrap returns the value if the option is Some.
                func (o Option[T]) Unwrap() T {{
                    if o.kind != some {{
                        panic(\"Option is None\")
                    }}
                    return o.val
                }}
                
                // Set sets the value and returns it.
                func (o *Option[T]) Set(val T) T {{
                    o.kind = some
                    o.val = val
                    return val
                }}
                
                // Unset sets the value to None.
                func (o *Option[T]) Unset() {{
                    o.kind = none
                }}
                
                // Some is a constructor for Option[T] which represents Some.
                func Some[T any](v T) Option[T] {{
                    return Option[T]{{
                        kind: some,
                        val:  v,
                    }}
                }}
                
                // None is a constructor for Option[T] which represents None.
                func None[T any]() Option[T] {{
                    return Option[T]{{
                        kind: none,
                    }}
                }}
                
                type ResultKind int
                
                const (
                    Ok ResultKind = iota
                    Err
                )
                
                type Result[T any, E any] struct {{
                    Kind ResultKind
                    Val  T
                    Err  E
                }}
                
                func (r Result[T, E]) IsOk() bool {{
                    return r.Kind == Ok
                }}
                
                func (r Result[T, E]) IsErr() bool {{
                    return r.Kind == Err
                }}
                
                func (r Result[T, E]) Unwrap() T {{
                    if r.Kind != Ok {{
                        panic(\"Result is Err\")
                    }}
                    return r.Val
                }}
                
                func (r Result[T, E]) UnwrapErr() E {{
                    if r.Kind != Err {{
                        panic(\"Result is Ok\")
                    }}
                    return r.Err
                }}
                
                func (r *Result[T, E]) Set(val T) T {{
                    r.Kind = Ok
                    r.Val = val
                    return val
                }}
                
                func (r *Result[T, E]) SetErr(err E) E {{
                    r.Kind = Err
                    r.Err = err
                    return err
                }}
                "
            );
            files.push(
                &format!("{}_types.go", world.name.to_kebab_case()),
                result_option_src.as_bytes(),
            );
        }
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut TinyGo,
    iface: &'a Interface,
    name: &'a str,
    export_funcs: Vec<(String, String)>,
    public_anonymous_types: BTreeSet<TypeId>,
}

impl InterfaceGenerator<'_> {
    fn get_typedef_target(&mut self, name: &str) -> String {
        format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        )
    }

    fn get_ty(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "uint8".into(),
            Type::U16 => "uint16".into(),
            Type::U32 => "uint32".into(),
            Type::U64 => "uint64".into(),
            Type::S8 => "int8".into(),
            Type::S16 => "int16".into(),
            Type::S32 => "int32".into(),
            Type::S64 => "int64".into(),
            Type::Float32 => "float32".into(),
            Type::Float64 => "float64".into(),
            Type::Char => "rune".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &self.iface().types[*id];
                match &ty.kind {
                    wit_bindgen_core::wit_parser::TypeDefKind::Type(ty) => self.get_ty(ty),
                    wit_bindgen_core::wit_parser::TypeDefKind::List(ty) => {
                        format!("[]{}", self.get_ty(ty))
                    }
                    wit_bindgen_core::wit_parser::TypeDefKind::Option(o) => {
                        self.gen.needs_result_option = true;
                        format!("Option[{}]", self.get_ty(o))
                    }
                    wit_bindgen_core::wit_parser::TypeDefKind::Result(r) => {
                        self.gen.needs_result_option = true;
                        format!(
                            "Result[{}, {}]",
                            self.get_optional_ty(r.ok.as_ref()),
                            self.get_optional_ty(r.err.as_ref())
                        )
                    }
                    _ => {
                        if let Some(name) = &ty.name {
                            self.get_typedef_target(name)
                        } else {
                            self.public_anonymous_types.insert(*id);
                            format!(
                                "{namespace}{name}",
                                namespace = self.name.to_upper_camel_case(),
                                name = self.get_ty_name(&Type::Id(*id)),
                            )
                        }
                    }
                }
            }
        }
    }

    fn print_ty(&mut self, _iface: &Interface, ty: &Type) {
        let ty = self.get_ty(ty);
        self.src.push_str(&ty);
    }

    fn get_c_ty(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::U8 => "uint8_t".into(),
            Type::U16 => "uint16_t".into(),
            Type::U32 => "uint32_t".into(),
            Type::U64 => "uint64_t".into(),
            Type::S8 => "int8_t".into(),
            Type::S16 => "int16_t".into(),
            Type::S32 => "int32_t".into(),
            Type::S64 => "int64_t".into(),
            Type::Float32 => "float".into(),
            Type::Float64 => "double".into(),
            Type::Char => "uint32_t".into(),
            Type::String => {
                format!(
                    "{namespace}_string_t",
                    namespace = self.gen.world.to_snake_case()
                )
            }
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                match &ty.name {
                    Some(name) => {
                        format!(
                            "{namespace}_{name}_t",
                            namespace = self.name.to_snake_case(),
                            name = name.to_snake_case(),
                        )
                    }
                    None => {
                        format!(
                            "{namespace}_{name}_t",
                            namespace = self.name.to_snake_case(),
                            name = self.get_c_ty_name(&Type::Id(*id)),
                        )
                    }
                }
            }
        }
    }

    fn get_ty_name(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "Bool".into(),
            Type::U8 => "U8".into(),
            Type::U16 => "U16".into(),
            Type::U32 => "U32".into(),
            Type::U64 => "U64".into(),
            Type::S8 => "S8".into(),
            Type::S16 => "S16".into(),
            Type::S32 => "S32".into(),
            Type::S64 => "S64".into(),
            Type::Float32 => "F32".into(),
            Type::Float64 => "F64".into(),
            Type::Char => "Byte".into(),
            Type::String => "String".into(),
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    return name.to_upper_camel_case();
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.get_ty(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(t) => {
                        let mut src = String::new();
                        src.push_str("Tuple");
                        src.push_str(&t.types.len().to_string());
                        for ty in t.types.iter() {
                            src.push_str(&self.get_ty_name(ty));
                        }
                        src.push('T');
                        src
                    }
                    TypeDefKind::Option(t) => {
                        let mut src = String::new();
                        src.push_str("Option");
                        src.push_str(&self.get_ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Result(r) => {
                        let mut src = String::new();
                        src.push_str("Result");
                        src.push_str(&self.get_optional_ty_name(r.ok.as_ref()));
                        src.push_str(&self.get_optional_ty_name(r.ok.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::List(t) => {
                        let mut src = String::new();
                        src.push_str("List");
                        src.push_str(&self.get_ty_name(t));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Future(t) => {
                        let mut src = String::new();
                        src.push_str("Future");
                        src.push_str(&self.get_optional_ty_name(t.as_ref()));
                        src.push('T');
                        src
                    }
                    TypeDefKind::Stream(t) => {
                        let mut src = String::new();
                        src.push_str("Stream");
                        src.push_str(&self.get_optional_ty_name(t.element.as_ref()));
                        src.push_str(&self.get_optional_ty_name(t.end.as_ref()));
                        src.push('T');
                        src
                    }
                }
            }
        }
    }

    fn get_optional_ty_name(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty_name(ty),
            None => "Empty".into(),
        }
    }

    fn get_c_ty_name(&mut self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".into(),
            Type::Char => "char32".into(),
            Type::U8 => "u8".into(),
            Type::S8 => "s8".into(),
            Type::U16 => "u16".into(),
            Type::S16 => "s16".into(),
            Type::U32 => "u32".into(),
            Type::S32 => "s32".into(),
            Type::U64 => "u64".into(),
            Type::S64 => "s64".into(),
            Type::Float32 => "float32".into(),
            Type::Float64 => "float64".into(),
            Type::String => "string".into(),
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    return name.to_snake_case();
                }
                match &ty.kind {
                    TypeDefKind::Type(t) => self.get_c_ty(t),
                    TypeDefKind::Record(_)
                    | TypeDefKind::Flags(_)
                    | TypeDefKind::Enum(_)
                    | TypeDefKind::Variant(_)
                    | TypeDefKind::Union(_) => {
                        unimplemented!()
                    }
                    TypeDefKind::Tuple(t) => {
                        let mut src = String::new();
                        src.push_str("tuple");
                        src.push_str(&t.types.len().to_string());
                        for ty in t.types.iter() {
                            src.push('_');
                            src.push_str(&self.get_c_ty_name(ty));
                        }
                        src
                    }
                    TypeDefKind::Option(ty) => {
                        format!("option_{}", self.get_c_ty_name(ty))
                    }
                    TypeDefKind::Result(r) => {
                        //imports_result_u32_u32_t
                        format!(
                            "result_{}_{}",
                            self.get_c_optional_type_name(r.ok.as_ref()),
                            self.get_c_optional_type_name(r.err.as_ref())
                        )
                    }
                    TypeDefKind::List(t) => {
                        format!("list_{}", self.get_c_ty_name(t))
                    }
                    TypeDefKind::Future(_t) => {
                        todo!()
                    }
                    TypeDefKind::Stream(_s) => {
                        todo!()
                    }
                }
            }
        }
    }

    fn get_c_optional_type_name(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_c_ty_name(ty),
            None => "void".into(),
        }
    }

    fn get_func_params(&mut self, _iface: &Interface, func: &Function) -> String {
        let mut params = String::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            params.push_str(name);
            params.push(' ');
            params.push_str(&self.get_ty(param));
        }
        params
    }

    fn get_func_results(&mut self, _iface: &Interface, func: &Function) -> String {
        let mut results = String::new();
        results.push(' ');
        match func.results.len() {
            0 => {}
            1 => {
                results.push_str(&self.get_ty(func.results.iter_types().next().unwrap()));
                results.push(' ');
            }
            _ => {
                results.push('(');
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        results.push_str(", ");
                    }
                    results.push_str(&self.get_ty(ty));
                }
                results.push_str(") ");
            }
        }
        results
    }

    fn print_c_result(&mut self, src: &mut Source, name: &str, param: &Type, in_import: bool) {
        let prefix = String::new();
        let mut param_name = String::new();
        let mut postfix = String::new();
        if in_import {
            if self.is_result_ty(param) {
                // add &err or &ret or both depend on result type
                match self.extract_result_ty(param) {
                    (None, None) => {}
                    (Some(_), Some(_)) => param_name.push_str("&ret, &err"),
                    _ => param_name.push_str("&ret"),
                };
                src.push_str(&format!("{prefix}{param_name}{postfix}"));
                return;
            }
        } else if self.is_result_ty(param) {
            match self.extract_result_ty(param) {
                (None, None) => (),
                (None, Some(err)) => {
                    param_name.push_str("err *C.");
                    postfix.push_str(&self.get_c_ty(&err));
                }
                (Some(ok), None) => {
                    param_name.push_str("ret *C.");
                    postfix.push_str(&self.get_c_ty(&ok));
                }
                (Some(ok), Some(err)) => {
                    param_name.push_str("ret *C.");
                    postfix.push_str(&self.get_c_ty(&ok));
                    postfix.push_str(", err *C.");
                    postfix.push_str(&self.get_c_ty(&err));
                }
            };
            src.push_str(&format!("{prefix}{param_name}{postfix}"));
            return;
        }
        self.print_c_param(src, name, param, in_import);
    }

    fn print_c_param(&mut self, src: &mut Source, name: &str, param: &Type, in_import: bool) {
        let pointer_prefix = if in_import { "&" } else { "*" };
        let mut prefix = String::new();
        let mut param_name = String::new();
        let mut postfix = String::new();

        if in_import {
            if self.is_arg_by_pointer(param) {
                prefix.push_str(pointer_prefix);
            }
            if name != "err" && name != "ret" {
                param_name = format!("lower_{name}");
            } else {
                param_name.push_str(name);
            }
        } else {
            postfix.push(' ');
            let maybe_option = self.extract_option_ty(param);
            param_name.push_str(name);
            if self.is_arg_by_pointer(param) || maybe_option.is_some() {
                postfix.push_str(pointer_prefix);
            }
            postfix.push_str("C.");
            let s = maybe_option
                .as_ref()
                .map(|o| self.get_c_ty(o))
                .unwrap_or(self.get_c_ty(param));
            postfix.push_str(&s);
        }
        src.push_str(&format!("{prefix}{param_name}{postfix}"));
    }

    fn print_c_func_params(
        &mut self,
        params: &mut Source,
        _iface: &Interface,
        func: &Function,
        in_import: bool,
    ) {
        // Append C params to source.
        //
        // If in_import is true, this function is invoked in `import_invoke` which uses `&` to dereference
        // argument of pointer type. The & is added as a prefix to the argument name. And there is no
        // type declaration needed to be added to the argument.
        //
        // If in_import is false, this function is invokved in printing export function signature.
        // It uses the form of `<param-name> *C.<param-type>` to print each parameter in the function, where
        // * is only used if the parameter is of pointer type.
        //
        // An exceptional case is the optional flattening rule. The rule only applies when
        // in_import is false. If the parameter is of type Option. It needs to flatten out the outer layer of
        // the option type and uses pointer for option's inner type. In this case, even if the inner type of
        // an option type is primitive, the * is still needed.

        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            self.print_c_param(params, name, param, in_import);
        }
    }

    fn print_c_func_results(
        &mut self,
        src: &mut Source,
        _iface: &Interface,
        func: &Function,
        in_import: bool,
    ) {
        let add_param_seperator = |src: &mut Source| {
            if !func.params.is_empty() {
                src.push_str(", ");
            }
        };
        match func.results.len() {
            0 => {
                // no return
                src.push_str(")");
            }
            1 => {
                // one return

                let add_bool_return = |src: &mut Source| {
                    if !in_import {
                        src.push_str(" bool");
                    }
                };
                let return_ty = func.results.iter_types().next().unwrap();
                if self.is_arg_by_pointer(return_ty) {
                    add_param_seperator(src);
                    self.print_c_result(src, "ret", return_ty, in_import);
                    src.push_str(")");
                } else {
                    src.push_str(")");
                    if !in_import {
                        src.push_str(&format!(" C.{ty}", ty = self.get_c_ty(return_ty)));
                    }
                }
                if self.is_result_ty(return_ty) || self.extract_option_ty(return_ty).is_some() {
                    add_bool_return(src);
                }
            }
            _n => {
                // multi-return
                add_param_seperator(src);
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        src.push_str(", ");
                    }
                    if in_import {
                        src.push_str(&format!("&ret{i}"));
                    } else {
                        src.push_str(&format!("ret{i} *C.{ty}", i = i, ty = self.get_c_ty(ty)));
                    }
                }
                src.push_str(")");
            }
        }
    }

    fn get_c_func_signature_helper(
        &mut self,
        iface: &Interface,
        func: &Function,
        in_import: bool,
    ) -> String {
        let mut src = Source::default();
        let name = if in_import {
            format!(
                "{}_{}",
                self.name.to_snake_case(),
                func.name.to_snake_case()
            )
        } else {
            format!(
                "{}{}",
                self.name.to_upper_camel_case(),
                func.name.to_upper_camel_case()
            )
        };

        if !in_import {
            src.push_str("func ");
        } else {
            src.push_str("C.");
        }
        src.push_str(&name);
        src.push_str("(");

        // prepare args
        self.print_c_func_params(&mut src, iface, func, in_import);

        // prepare returns
        self.print_c_func_results(&mut src, iface, func, in_import);
        src.to_string()
    }

    fn get_func_signature_no_interface(&mut self, iface: &Interface, func: &Function) -> String {
        format!(
            "{}({}){}",
            func.name.to_upper_camel_case(),
            self.get_func_params(iface, func),
            self.get_func_results(iface, func)
        )
    }

    fn print_func_signature(&mut self, iface: &Interface, func: &Function) {
        self.src.push_str("func ");
        let func_name = self.name.to_upper_camel_case();
        self.src.push_str(&func_name);
        let func_sig = self.get_func_signature_no_interface(iface, func);
        self.src.push_str(&func_sig);
        self.src.push_str("{\n");
    }

    fn get_field_name(&mut self, field: &Field) -> String {
        field.name.to_upper_camel_case()
    }

    fn is_arg_by_pointer(&self, ty: &Type) -> bool {
        // TODO: can reuse this for other things
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Type(t) => self.is_arg_by_pointer(t),
                TypeDefKind::Variant(_) => true,
                TypeDefKind::Union(_) => true,
                TypeDefKind::Option(_) => true,
                TypeDefKind::Result(_) => true,
                TypeDefKind::Enum(_) => false,
                TypeDefKind::Flags(_) => false,
                TypeDefKind::Tuple(_) | TypeDefKind::Record(_) | TypeDefKind::List(_) => true,
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
            },
            Type::String => true,
            _ => false,
        }
    }

    fn extract_option_ty(&self, ty: &Type) -> Option<Type> {
        // TODO: don't copy from the C code
        // optional param pointer flattening
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Option(o) => Some(o.to_owned()),
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => None,
            },
            _ => None,
        }
    }

    fn is_not_option(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Option(_o) => false,
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => true,
            },
            _ => true,
        }
    }

    fn extract_result_ty(&self, ty: &Type) -> (Option<Type>, Option<Type>) {
        //TODO: don't copy from the C code
        // optimization on the C size.
        // See https://github.com/bytecodealliance/wit-bindgen/pull/450
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Result(r) => (r.ok, r.err),
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => (None, None),
            },
            _ => (None, None),
        }
    }

    fn is_result_ty(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Result(_) => true,
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => false,
            },
            _ => false,
        }
    }

    fn get_optional_ty(&mut self, ty: Option<&Type>) -> String {
        match ty {
            Some(ty) => self.get_ty(ty),
            None => "struct{}".into(),
        }
    }

    fn print_anonymous_type(&mut self, ty: TypeId) {
        let kind = &self.iface.types[ty].kind;
        match kind {
            TypeDefKind::Type(_)
            | TypeDefKind::Flags(_)
            | TypeDefKind::Record(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Variant(_)
            | TypeDefKind::Union(_) => {
                unreachable!()
            }
            TypeDefKind::Tuple(t) => {
                let name = format!(
                    "{}{}",
                    self.name.to_upper_camel_case(),
                    self.get_ty_name(&Type::Id(ty))
                );
                self.src.push_str(&format!("type {name} struct {{\n",));
                for (i, ty) in t.types.iter().enumerate() {
                    let ty = self.get_ty(ty);
                    self.src.push_str(&format!("   F{i} {ty}\n",));
                }
                self.src.push_str("}\n\n");
            }
            TypeDefKind::Option(_t) => {}
            TypeDefKind::Result(_r) => {}
            TypeDefKind::List(_l) => {}
            TypeDefKind::Future(_) => todo!("print_anonymous_type for future"),
            TypeDefKind::Stream(_) => todo!("print_anonymous_type for stream"),
        }
    }

    fn is_empty_type(&self, ty: &Type) -> bool {
        // TODO: reuse from C
        let id = match ty {
            Type::Id(id) => *id,
            _ => return false,
        };
        match &self.iface.types[id].kind {
            TypeDefKind::Type(t) => self.is_empty_type(t),
            TypeDefKind::Record(r) => r.fields.is_empty(),
            TypeDefKind::Tuple(t) => t.types.is_empty(),
            _ => false,
        }
    }

    fn get_nonempty_type<'o>(&self, ty: Option<&'o Type>) -> Option<&'o Type> {
        // TODO: reuse from C
        ty.filter(|&ty| !self.is_empty_type(ty))
    }

    fn get_primitive_type_value(&self, ty: &Type) -> String {
        if self.is_arg_by_pointer(ty) {
            "nil".into()
        } else {
            match ty {
                Type::Bool => "false".into(),
                _ => "0".into(),
            }
        }
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        // lower params to c
        func.params.iter().for_each(|(name, ty)| {
            func_bindgen.lower(name, ty, false, false);
        });
        // lift results from c
        match func.results.len() {
            0 => {}
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lift("ret", ty, false, false);
            }
            _ => {
                for (i, ty) in func.results.iter_types().enumerate() {
                    func_bindgen.lift(&format!("ret{i}"), ty, false, true);
                }
            }
        };
        let c_args = func_bindgen.c_args;
        let ret = func_bindgen.args;
        let lower_src = func_bindgen.lower_src.to_string();
        let lift_src = func_bindgen.lift_src.to_string();

        // // print function signature
        self.print_func_signature(iface, func);

        // body
        // prepare args
        self.src.push_str(lower_src.as_str());

        self.import_invoke(iface, func, c_args, &lift_src, ret);

        // return

        self.src.push_str("}\n\n");
    }

    fn import_invoke(
        &mut self,
        iface: &Interface,
        func: &Function,
        _c_args: Vec<String>,
        lift_src: &str,
        ret: Vec<String>,
    ) {
        let invoke = self.get_c_func_signature_helper(iface, func, true);
        match func.results.len() {
            0 => {
                self.src.push_str(&invoke);
                self.src.push_str("\n");
            }
            1 => {
                let return_ty = func.results.iter_types().next().unwrap();
                if self.is_arg_by_pointer(return_ty) {
                    if !self.is_result_ty(return_ty) {
                        let optional_type = self.extract_option_ty(return_ty);
                        // flatten optional type or use return type #https://github.com/bytecodealliance/wit-bindgen/pull/453
                        // TODO: reuse from C
                        let c_ret_type = optional_type
                            .as_ref()
                            .map(|o| self.get_c_ty(o))
                            .unwrap_or_else(|| self.get_c_ty(return_ty));
                        self.src.push_str(&format!("var ret C.{c_ret_type}\n"));
                    } else {
                        let (ok, err) = self.extract_result_ty(return_ty);
                        let c_ret_type = match (ok, err) {
                            (None, Some(err)) => self.get_c_ty(&err),
                            (Some(ok), None) => self.get_c_ty(&ok),
                            (Some(ok), Some(err)) => {
                                let c_err_type = self.get_c_ty(&err);
                                self.src.push_str(&format!("var err C.{c_err_type}\n"));
                                self.get_c_ty(&ok)
                            }
                            _ => "void".to_string(),
                        };
                        if c_ret_type != *"void" {
                            self.src.push_str(&format!("var ret C.{c_ret_type}\n"));
                            self.src
                                .push_str(&format!("var empty_ret C.{c_ret_type}\n"));
                        }
                    };
                    self.src.push_str(&invoke);
                    self.src.push_str("\n");
                } else {
                    self.src.push_str(&format!("ret := {invoke}\n"));
                }
                self.src.push_str(lift_src);
                self.src.push_str(&format!("return {ret}\n", ret = ret[0]));
            }
            _n => {
                for (i, ty) in func.results.iter_types().enumerate() {
                    let ty = self.get_c_ty(ty);
                    self.src.push_str(&format!("var ret{i} C.{ty}\n"));
                }
                self.src.push_str(&invoke);
                self.src.push_str("\n");
                self.src.push_str(lift_src);
                self.src.push_str("return ");
                for (i, _) in func.results.iter_types().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.src.push_str(&format!("lift_ret{i}"));
                }
                self.src.push_str("\n");
            }
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        match func.results.len() {
            0 => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(name, ty, true, false);
                });
            }
            1 => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(name, ty, true, false);
                });
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lower("result", ty, true, false);
            }
            _ => {
                func.params.iter().for_each(|(name, ty)| {
                    func_bindgen.lift(name, ty, true, true);
                });
                for (i, ty) in func.results.iter_types().enumerate() {
                    func_bindgen.lower(&format!("result{i}"), ty, true, true);
                }
            }
        };

        let args = func_bindgen.args;
        let ret = func_bindgen.c_args;
        let lift_src = func_bindgen.lift_src.to_string();
        let lower_src = func_bindgen.lower_src.to_string();

        let interface_method_decl = self.get_func_signature_no_interface(iface, func);
        let export_func = {
            let mut src = String::new();
            // header
            src.push_str("//export ");
            let name = format!(
                "{}_{}",
                self.name.to_snake_case(),
                func.name.to_snake_case()
            );
            src.push_str(&name);
            src.push('\n');

            // signature
            src.push_str(&self.get_c_func_signature_helper(iface, func, false));
            src.push_str(" {\n");

            // src.push_str(&self.get_c_func_impl(iface, func));
            // prepare args

            src.push_str(&lift_src);

            // invoke
            let invoke = format!(
                "{}.{}({})",
                &self.name.to_snake_case(),
                &func.name.to_upper_camel_case(),
                args.iter()
                    .enumerate()
                    .map(|(i, name)| format!(
                        "{}{}",
                        name,
                        if i < func.params.len() - 1 { ", " } else { "" }
                    ))
                    .collect::<String>()
            );

            // prepare ret
            match func.results.len() {
                0 => {
                    src.push_str(&format!("{invoke}\n"));
                }
                1 => {
                    let return_ty = func.results.iter_types().next().unwrap();
                    if self.is_empty_type(return_ty) {
                        src.push_str(&format!("{invoke}\n"));
                    } else {
                        src.push_str(&format!("result := {invoke}\n"));
                    }
                    src.push_str(&lower_src);

                    let lower_result = &ret[0];
                    if self.is_arg_by_pointer(return_ty) {
                        // flatten result type
                        if self.is_result_ty(return_ty) {
                        } else if let Some(_o) = self.extract_option_ty(return_ty) {
                            uwriteln!(
                                src,
                                "
                                *ret = {lower_result}
                                return result.IsSome()"
                            )
                        } else {
                            src.push_str(&format!("*ret = {lower_result}\n"));
                        }
                    } else {
                        src.push_str(&format!("return {ret}\n", ret = &ret[0]));
                    }
                }
                _ => {
                    for i in 0..func.results.len() {
                        if i > 0 {
                            src.push_str(", ")
                        }
                        src.push_str(&format!("result{i}"));
                    }
                    src.push_str(&format!(" := {invoke}\n"));
                    src.push_str(&lower_src);
                    for (i, lower_result) in ret.iter().enumerate() {
                        src.push_str(&format!("*ret{i} = {lower_result}\n"));
                    }
                }
            };

            src.push_str("\n}\n");
            src
        };
        self.gen
            .export_funcs
            .push((interface_method_decl, export_func));
    }

    fn finish(&mut self) {
        while let Some(ty) = self.public_anonymous_types.pop_last() {
            self.print_anonymous_type(ty);
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        record: &wit_bindgen_core::wit_parser::Record,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        self.src.push_str(&format!("type {name} struct {{\n",));
        for field in record.fields.iter() {
            let ty = self.get_ty(&field.ty);
            let name = self.get_field_name(field);
            self.src.push_str(&format!("   {name} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_flags(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        flags: &wit_bindgen_core::wit_parser::Flags,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        // TODO: use flags repr to determine how many flags are needed
        self.src.push_str(&format!("type {name} uint64\n"));
        self.src.push_str("const (\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            if i == 0 {
                self.src.push_str(&format!(
                    "   {name}_{flag} {name} = 1 << iota\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            } else {
                self.src.push_str(&format!(
                    "   {name}_{flag}\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            }
        }
        self.src.push_str(")\n\n");
    }

    fn type_tuple(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _tuple: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = self.get_ty_name(&Type::Id(id));
        self.src.push_str(&ty);
    }

    fn type_variant(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        variant: &wit_bindgen_core::wit_parser::Variant,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str(&"const (\n".to_string());

        for (i, case) in variant.cases.iter().enumerate() {
            let case_name = case.name.to_upper_camel_case();
            if i == 0 {
                self.src
                    .push_str(&format!("   {name}Kind{case_name} {name}Kind = iota\n",));
            } else {
                self.src.push_str(&format!("   {name}Kind{case_name}\n",));
            }
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str(&"val any\n".to_string());
        self.src.push_str("}\n\n");

        uwriteln!(
            self.src,
            "func (n {name}) Kind() {name}Kind {{
                return n.kind
            }}
            "
        );

        for case in variant.cases.iter() {
            let case_name = case.name.to_upper_camel_case();
            if let Some(ty) = self.get_nonempty_type(case.ty.as_ref()) {
                self.gen.needs_fmt_import = true;
                let ty = self.get_ty(ty);
                uwriteln!(
                    self.src,
                    "func {name}{case_name}(v {ty}) {name} {{
                        return {name}{{kind: {name}Kind{case_name}, val: v}}
                    }}
                    ",
                );
                uwriteln!(
                    self.src,
                    "func (n {name}) Get{case_name}() {ty} {{
                        if g, w := n.Kind(), {name}Kind{case_name}; g != w {{
                            panic(fmt.Sprintf(\"Attr kind is %v, not %v\", g, w))
                        }}
                        return n.val.({ty})
                    }}
                    ",
                );
                uwriteln!(
                    self.src,
                    "func (n {name}) Set{case_name}(v {ty}) {{
                        n.val = v
                        n.kind = {name}Kind{case_name}
                    }}
                    ",
                );
            } else {
                uwriteln!(
                    self.src,
                    "func {name}{case_name}() {name} {{
                        return {name}{{kind: {name}Kind{case_name}}}
                    }}
                    ",
                );
            }
        }
    }

    fn type_option(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        self.get_ty(&Type::Id(id));
    }

    fn type_result(
        &mut self,
        id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        self.get_ty(&Type::Id(id));
    }

    fn type_union(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        union: &wit_bindgen_core::wit_parser::Union,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str(&"const (\n".to_string());

        for (i, _case) in union.cases.iter().enumerate() {
            let case_name = format!("F{i}");
            if i == 0 {
                self.src
                    .push_str(&format!("   {name}Kind{case_name} {name}Kind = iota\n",));
            } else {
                self.src.push_str(&format!("   {name}Kind{case_name}\n",));
            }
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str(&"val any\n".to_string());
        self.src.push_str("}\n\n");

        uwriteln!(
            self.src,
            "func (n {name}) Kind() {name}Kind {{
                return n.kind
            }}
            "
        );

        for (i, case) in union.cases.iter().enumerate() {
            self.gen.needs_fmt_import = true;
            let ty = self.get_ty(&case.ty);
            let case_name = format!("F{i}");
            uwriteln!(
                self.src,
                "func {name}F{i}(v {ty}) {name} {{
                    return {name}{{kind: {name}Kind{case_name}, val: v}}
                }}
                ",
            );
            uwriteln!(
                self.src,
                "func (n {name}) Get{case_name}() {ty} {{
                    if g, w := n.Kind(), {name}Kind{case_name}; g != w {{
                        panic(fmt.Sprintf(\"Attr kind is %v, not %v\", g, w))
                    }}
                    return n.val.({ty})
                }}
                ",
            );
            uwriteln!(
                self.src,
                "func (n {name}) Set{case_name}(v {ty}) {{
                    n.val = v
                    n.kind = {name}Kind{case_name}
                }}
                ",
            );
        }
    }

    fn type_enum(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        enum_: &wit_bindgen_core::wit_parser::Enum,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        // TODO: use variant's tag to determine how many cases are needed
        // this will help to optmize the Kind type.
        self.src.push_str(&format!("type {name}Kind int\n\n"));
        self.src.push_str(&"const (\n".to_string());

        for (i, case) in enum_.cases.iter().enumerate() {
            let case_name = case.name.to_upper_camel_case();
            if i == 0 {
                self.src
                    .push_str(&format!("   {name}Kind{case_name} {name}Kind = iota\n",));
            } else {
                self.src.push_str(&format!("   {name}Kind{case_name}\n",));
            }
        }
        self.src.push_str(")\n\n");

        self.src.push_str(&format!("type {name} struct {{\n"));
        self.src.push_str(&format!("kind {name}Kind\n"));
        self.src.push_str("}\n\n");

        uwriteln!(
            self.src,
            "func (n {name}) Kind() {name}Kind {{
                return n.kind
            }}
            "
        );

        for case in enum_.cases.iter() {
            let case_name = case.name.to_upper_camel_case();
            uwriteln!(
                self.src,
                "func {name}{case_name}() {name} {{
                    return {name}{{kind: {name}Kind{case_name}}}
                }}
                ",
            );
        }
    }

    fn type_alias(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_list(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        let ty = self.get_ty(ty);
        self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_builtin(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!("type_builtin")
    }
}

struct FunctionBindgen<'a, 'b> {
    interface: &'a mut InterfaceGenerator<'b>,
    _func: &'a Function,
    c_args: Vec<String>,
    args: Vec<String>,
    lower_src: Source,
    lift_src: Source,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(interface: &'a mut InterfaceGenerator<'b>, func: &'a Function) -> Self {
        Self {
            interface,
            _func: func,
            c_args: Vec::new(),
            args: Vec::new(),
            lower_src: Source::default(),
            lift_src: Source::default(),
        }
    }

    fn lower(&mut self, name: &str, ty: &Type, in_export: bool, multi_return: bool) {
        let lower_name = format!("lower_{name}");
        let count = if multi_return { 1 } else { 0 };
        self.lower_value(
            name,
            ty,
            lower_name.as_ref(),
            self.interface.extract_option_ty(ty).is_some(),
            count,
            in_export,
        );
        self.c_args.push(lower_name);
    }

    fn lower_value(
        &mut self,
        param: &str,
        ty: &Type,
        lower_name: &str,
        flatten: bool,
        count: u32,
        in_export: bool,
    ) {
        if self.interface.is_empty_type(ty) {
            return;
        }
        match ty {
            Type::Bool => {
                self.lower_src
                    .push_str(&format!("{lower_name} := {param}\n",));
            }
            Type::String => {
                uwriteln!(
                    self.lower_src,
                    "var {lower_name} C.{value}",
                    value = self.interface.get_c_ty(ty),
                );
                uwrite!(
                    self.lower_src,
                    "
                    {lower_name}.ptr = C.CString({param})
                    {lower_name}.len = C.size_t(len({param}))
                    "
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.iface.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        for field in r.fields.iter() {
                            let c_field_name = &self.get_c_field_name(field);
                            let field_name = &self.interface.get_field_name(field);

                            self.lower_value(
                                &format!("{param}.{field_name}"),
                                &field.ty,
                                &format!("{lower_name}_{c_field_name}"),
                                false,
                                count + 1,
                                in_export,
                            );
                            uwrite!(
                                self.lower_src,
                                "
                                {lower_name}.{c_field_name} = {lower_name}_{c_field_name}
                                "
                            )
                        }
                    }

                    TypeDefKind::Flags(f) => {
                        let int_repr = c_int_repr(flags_repr(f));
                        self.lower_src
                            .push_str(&format!("{lower_name} := C.{int_repr}({param})\n"));
                    }
                    TypeDefKind::Tuple(t) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id)); // okay to unwrap because a record must have a name
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        for (i, ty) in t.types.iter().enumerate() {
                            self.lower_value(
                                &format!("{param}.F{i}"),
                                ty,
                                &format!("{lower_name}_f{i}"),
                                false,
                                count + 1,
                                in_export,
                            );

                            uwrite!(
                                self.lower_src,
                                "
                                {lower_name}.f{i} = {lower_name}_f{i}
                                "
                            );
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let c_typedef_target = if flatten {
                            self.interface.get_c_ty(o)
                        } else {
                            self.interface.get_c_ty(&Type::Id(*id))
                        };
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        self.lower_src
                            .push_str(&format!("if {param}.IsSome() {{\n"));
                        self.lower_value(
                            &format!("{param}.Unwrap()"),
                            o,
                            &format!("{lower_name}_val"),
                            flatten,
                            count + 1,
                            in_export,
                        );
                        if self.interface.is_not_option(o) && flatten {
                            if !self.interface.is_empty_type(o) {
                                self.lower_src
                                    .push_str(&format!("{lower_name} = {lower_name}_val\n"));
                            }
                        } else {
                            if !self.interface.is_empty_type(o) {
                                self.lower_src
                                    .push_str(&format!("{lower_name}.val = {lower_name}_val\n"));
                            }
                            self.lower_src
                                .push_str(&format!("{lower_name}.is_some = true\n"));
                        }
                        self.lower_src.push_str("}\n");
                    }
                    TypeDefKind::Result(r) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        self.interface.gen.needs_import_unsafe = true;

                        if count > 0 || !in_export {
                            // import
                            self.lower_src
                                .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                            self.lower_src
                                .push_str(&format!("{lower_name}.is_err = {param}.IsErr()\n"));
                            match (r.ok, r.err) {
                                (None, Some(err)) => {
                                    let err = self.interface.get_ty(&err);
                                    uwriteln!(
                                            self.lower_src,
                                            "
                                            {lower_name}_ptr := (*{err})(unsafe.Pointer(&{lower_name}.val))
                                            if {param}.IsErr() {{
                                                *{lower_name}_ptr = {param}.UnwrapErr()
                                            }}"
                                        );
                                }
                                (Some(ok), None) => {
                                    let ok = self.interface.get_ty(&ok);
                                    uwriteln!(
                                        self.lower_src,
                                        "
                                        {lower_name}_ptr := (*{ok})(unsafe.Pointer(&{lower_name}.val))
                                        if {param}.IsOk() {{
                                            *{lower_name}_ptr = {param}.Unwrap()
                                        }}"
                                    );
                                }
                                (Some(ok), Some(err)) => {
                                    let ok = self.interface.get_ty(&ok);
                                    let err = self.interface.get_ty(&err);
                                    uwriteln!(
                                        self.lower_src,
                                        "
                                        if {param}.IsOk() {{
                                            {lower_name}_ptr := (*{ok})(unsafe.Pointer(&{lower_name}.val))
                                            *{lower_name}_ptr = {param}.Unwrap()
                                        }} else {{
                                            {lower_name}_ptr := (*{err})(unsafe.Pointer(&{lower_name}.val))
                                            *{lower_name}_ptr = {param}.UnwrapErr()
                                        }}"
                                    );
                                }
                                _ => {}
                            }
                        } else {
                            match (r.ok, r.err) {
                                (None, None) => self.lower_src.push_str("return result.IsOk()"),
                                (None, Some(err)) => {
                                    let err = self.interface.get_ty(&err);
                                    uwriteln!(
                                        self.lower_src,
                                        "
                                        if {param}.IsErr() {{
                                            err_ptr := (*{err})(unsafe.Pointer(&err))
                                            *err_ptr = {param}.UnwrapErr()
                                        }}
                                        return result.IsOk()"
                                    );
                                }
                                (Some(ok), None) => {
                                    let ok = self.interface.get_ty(&ok);
                                    uwriteln!(
                                        self.lower_src,
                                        "
                                        if {param}.IsOk() {{
                                            ret_ptr := (*{ok})(unsafe.Pointer(&ret))
                                            *ret_ptr = {param}.Unwrap()
                                        }}
                                        return result.IsOk()"
                                    );
                                }
                                (Some(ok), Some(err)) => {
                                    let ok = self.interface.get_ty(&ok);
                                    let err = self.interface.get_ty(&err);
                                    uwriteln!(
                                        self.lower_src,
                                        "
                                        if {param}.IsOk() {{
                                            ret_ptr := (*{ok})(unsafe.Pointer(&ret))
                                            *ret_ptr = {param}.Unwrap()
                                        }} else {{
                                            err_ptr := (*{err})(unsafe.Pointer(&err))
                                            *err_ptr = {param}.UnwrapErr()
                                        }}
                                        return result.IsOk()"
                                    );
                                }
                            }
                        }
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        let list_ty = self.interface.get_c_ty(l);
                        uwrite!(
                            self.lower_src,
                            "
                            if len({param}) == 0 {{
                                {lower_name}.ptr = nil
                                {lower_name}.len = 0
                            }} else {{
                                {lower_name}.ptr = (*C.{list_ty})(unsafe.Pointer(&{param}[0]))
                                {lower_name}.len = C.size_t(len({param}))
                            }}
                            "
                        );
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lower_src,
                            "var {lower_name} C.{value}",
                            value = self.interface.get_c_ty(t),
                        );
                        self.lower_value(
                            param,
                            t,
                            &format!("{lower_name}_val"),
                            flatten,
                            count + 1,
                            in_export,
                        );
                        uwrite!(
                            self.lower_src,
                            "
                            {lower_name} = {lower_name}_val
                            "
                        );
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;

                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lower_src.push_str(&format!(
                                "if {param}.Kind() == {ty}Kind{case_name} {{\n"
                            ));
                            if let Some(ty) = self.interface.get_nonempty_type(case.ty.as_ref()) {
                                let name = self.interface.get_ty(ty);
                                uwriteln!(
                                    self.lower_src,
                                    "
                                    {lower_name}.tag = {i}
                                    {lower_name}_ptr := (*{name})(unsafe.Pointer(&{lower_name}.val))
                                    *{lower_name}_ptr = {param}.Get{case_name}()"
                                );
                            } else {
                                uwriteln!(
                                    self.lower_src,
                                    "
                                    {lower_name}.tag = {i}"
                                );
                            }
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lower_src.push_str(&format!(
                                "if {param}.Kind() == {ty}Kind{case_name} {{\n"
                            ));
                            uwriteln!(
                                self.lower_src,
                                "
                                {lower_name} = {i}"
                            );
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Union(u) => {
                        self.interface.gen.needs_import_unsafe = true;

                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        for (i, case) in u.cases.iter().enumerate() {
                            let case_name = format!("F{i}");
                            self.lower_src.push_str(&format!(
                                "if {param}.Kind() == {ty}Kind{case_name} {{\n"
                            ));
                            let name = self.interface.get_ty(&case.ty);
                            uwriteln!(
                                self.lower_src,
                                "
                                {lower_name}.tag = {i}
                                {lower_name}_ptr := (*{name})(unsafe.Pointer(&{lower_name}.val))
                                *{lower_name}_ptr = {param}.Get{case_name}()"
                            );
                            self.lower_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl future"),
                    _ => self.lower_src.push_str(""),
                }
            }
            a => {
                self.lower_src.push_str(&format!(
                    "{lower_name} := C.{c_type_name}({param_name})\n",
                    c_type_name = self.interface.get_c_ty(a),
                    param_name = param,
                ));
            }
        }
    }

    fn lift(&mut self, name: &str, ty: &Type, in_export: bool, multi_return: bool) {
        let lift_name = format!("lift_{name}");
        if multi_return {
            self.lift_value(name, ty, lift_name.as_str(), false, 1, in_export);
        } else if self.interface.extract_option_ty(ty).is_some() {
            self.lift_value(name, ty, lift_name.as_str(), true, 0, in_export);
        } else {
            self.lift_value(name, ty, lift_name.as_str(), false, 0, in_export);
        }
        self.args.push(lift_name);
    }

    fn lift_value(
        &mut self,
        param: &str,
        ty: &Type,
        lift_name: &str,
        flatten: bool,
        count: u32,
        in_export: bool,
    ) {
        match ty {
            Type::Bool => {
                self.lift_src
                    .push_str(&format!("{lift_name} := {param}\n",));
            }
            Type::String => {
                uwriteln!(
                    self.lift_src,
                    "var {name} {value}
                    ",
                    name = lift_name,
                    value = self.interface.get_ty(ty),
                );
                uwriteln!(
                    self.lift_src,
                    "{lift_name} = C.GoStringN({param}.ptr, C.int({param}.len))"
                );
            }
            Type::Id(id) => {
                let ty = &self.interface.iface.types[*id]; // receive type
                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}
                            ",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for field in r.fields.iter() {
                            let field_name = &self.interface.get_field_name(field);
                            let c_field_name = &self.get_c_field_name(field);
                            self.lift_value(
                                &format!("{param}.{c_field_name}"),
                                &field.ty,
                                &format!("{lift_name}_{field_name}"),
                                false,
                                count + 1,
                                in_export,
                            );
                            self.lift_src.push_str(&format!(
                                "{lift_name}.{field_name} = {lift_name}_{field_name}\n"
                            ));
                        }
                    }
                    TypeDefKind::Flags(_f) => {
                        let field = self.interface.get_typedef_target(ty.name.as_ref().unwrap());
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}
                            ",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        self.lift_src
                            .push_str(&format!("{lift_name} = {field}({param})\n"))
                    }
                    TypeDefKind::Tuple(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}
                            ",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        for (i, t) in t.types.iter().enumerate() {
                            self.lift_value(
                                &format!("{param}.f{i}"),
                                t,
                                &format!("{lift_name}_F{i}"),
                                false,
                                count + 1,
                                in_export,
                            );
                            self.lift_src
                                .push_str(&format!("{lift_name}.F{i} = {lift_name}_F{i}\n"));
                        }
                    }
                    TypeDefKind::Option(o) => {
                        let lift_type = self.interface.get_ty(&Type::Id(*id));
                        self.lift_src
                            .push_str(&format!("var {lift_name} {lift_type}\n"));
                        // `flatten` will be true if the top level type is an option and hasn't
                        // been flattened yet.
                        if self.interface.is_not_option(o) && flatten {
                            // if the type is Option[T] where T is primitive, this is a special
                            // case where the primitive is a pointer type. Hence the `param` needs
                            // to be dereferenced.

                            // It only happens the type has just one level of option. It has more levels,
                            // the `param` will not need a * dereference.

                            // TODO: please simplfy this logic
                            let is_pointer = self.interface.is_arg_by_pointer(o);
                            let val = self.interface.get_primitive_type_value(o);
                            let param = if !in_export {
                                if is_pointer {
                                    let c_target_name = self.interface.get_c_ty(o);
                                    self.lift_src.push_str(&format!(
                                        "var {lift_name}_c C.{c_target_name}\n"
                                    ));
                                    self.lift_src
                                        .push_str(&format!("if {param} == {lift_name}_c {{\n"));
                                } else {
                                    self.lift_src.push_str(&format!("if {param} == {val} {{\n"));
                                }
                                param.to_string()
                            } else {
                                let need_pointer = (count == 0) || count > 0 && is_pointer;
                                if need_pointer {
                                    self.lift_src.push_str(&format!("if {param} == nil {{\n"));
                                } else {
                                    self.lift_src.push_str(&format!("if {param} == {val} {{\n"));
                                }
                                let need_pointer_in_param = count == 0 && !is_pointer;
                                if need_pointer_in_param {
                                    format!("*{param}")
                                } else {
                                    param.to_string()
                                }
                            };
                            self.lift_src.push_str(&format!("{lift_name}.Unset()\n"));
                            self.lift_src.push_str("} else {\n");
                            self.lift_value(
                                &param,
                                o,
                                &format!("{lift_name}_val"),
                                flatten,
                                count + 1,
                                in_export,
                            );
                            self.lift_src
                                .push_str(&format!("{lift_name}.Set({lift_name}_val)\n"));
                            self.lift_src.push_str("}\n");
                        } else {
                            self.lift_src.push_str(&format!("if {param}.is_some {{\n"));

                            self.lift_value(
                                &format!("{param}.val"),
                                o,
                                &format!("{lift_name}_val"),
                                flatten,
                                count + 1,
                                in_export,
                            );

                            self.lift_src
                                .push_str(&format!("{lift_name}.Set({lift_name}_val)\n"));
                            self.lift_src.push_str("} else {\n");
                            self.lift_src.push_str(&format!("{lift_name}.Unset()\n"));
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Result(r) => {
                        self.interface.gen.needs_result_option = true;
                        let ty = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}
                            ",
                            name = lift_name,
                            value = ty,
                        );
                        match (r.ok, r.err) {
                            (None, Some(err)) => {
                                let err = self.interface.get_ty(&err);
                                if in_export || count > 0 {
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{err})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                } else {
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{err})(unsafe.Pointer(&ret))\n"
                                    ));
                                    self.lift_src
                                        .push_str(&format!("if {param} == empty_{param} {{ \n"));
                                }
                                self.lift_src
                                    .push_str(&format!("{lift_name}.SetErr(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str("} else {\n");
                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(struct{{}}{{}})\n"));
                                self.lift_src.push_str("}\n");
                            }
                            (Some(ok), None) => {
                                let ok = self.interface.get_ty(&ok);
                                if in_export || count > 0 {
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{ok})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                } else {
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{ok})(unsafe.Pointer(&ret))\n"
                                    ));
                                    self.lift_src
                                        .push_str(&format!("if {param} == empty_{param} {{ \n"));
                                }
                                self.lift_src
                                    .push_str(&format!("{lift_name}.SetErr(struct{{}}{{}})\n"));
                                self.lift_src.push_str("} else {\n");
                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str("}\n");
                            }
                            (Some(ok), Some(err)) => {
                                let ok = self.interface.get_ty(&ok);
                                let err = self.interface.get_ty(&err);

                                if in_export || count > 0 {
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{err})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}.SetErr(*{lift_name}_ptr)\n"
                                    ));
                                    self.lift_src.push_str("} else {\n");
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{ok})(unsafe.Pointer(&{param}.val))\n"));
                                } else {
                                    self.lift_src
                                        .push_str(&format!("if {param} == empty_{param} {{ \n"));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{err})(unsafe.Pointer(&err))\n"
                                    ));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}.SetErr(*{lift_name}_ptr)\n"
                                    ));
                                    self.lift_src.push_str("} else {\n");
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{ok})(unsafe.Pointer(&ret))\n"
                                    ));
                                }

                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str("}\n");
                            }
                            _ => {
                                self.lift_src.push_str(&format!(
                                    "{lift_name} = Result[struct{{}}, struct{{}}] {{}}\n"
                                ));
                            }
                        }
                    }
                    TypeDefKind::List(l) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let list_ty = self.interface.get_ty(l);
                        uwriteln!(
                            self.lift_src,
                            "var {name} {value}
                            ",
                            name = lift_name,
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        uwriteln!(self.lift_src, "
                            if {param}.len == 0 {{
                                {lift_name} = nil
                            }} else {{
                                {lift_name} = (*[1 << 28]{list_ty})(unsafe.Pointer({param}.ptr))[:{param}.len:{param}.len]
                            }}");
                    }
                    TypeDefKind::Type(t) => {
                        uwriteln!(
                            self.lift_src,
                            "var {lift_name} {value}
                            ",
                            value = self.interface.get_ty(&Type::Id(*id)),
                        );
                        // let c_field_name = &self.get_c_field_name(field);
                        self.lift_value(
                            param,
                            t,
                            &format!("{lift_name}_val"),
                            flatten,
                            count + 1,
                            in_export,
                        );
                        self.lift_src
                            .push_str(&format!("{lift_name} = {lift_name}_val\n"));
                    }
                    TypeDefKind::Variant(v) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in v.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            if let Some(ty) = self.interface.get_nonempty_type(case.ty.as_ref()) {
                                let ty = self.interface.get_ty(ty);
                                uwriteln!(
                                    self.lift_src,
                                    "
                                    {lift_name}_ptr := (*{ty})(unsafe.Pointer(&{param}.val))
                                    {name}{case_name}(*{lift_name}_ptr)"
                                );
                            } else {
                                uwriteln!(
                                    self.lift_src,
                                    "
                                    {name}{case_name}()"
                                );
                            }
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Enum(e) => {
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in e.cases.iter().enumerate() {
                            let case_name = case.name.to_upper_camel_case();
                            self.lift_src.push_str(&format!("if {param} == {i} {{\n"));
                            uwriteln!(
                                self.lift_src,
                                "
                                {name}{case_name}()"
                            );
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Union(u) => {
                        self.interface.gen.needs_import_unsafe = true;
                        let name = self.interface.get_ty(&Type::Id(*id));
                        uwriteln!(self.lift_src, "var {lift_name} {name}");
                        for (i, case) in u.cases.iter().enumerate() {
                            self.lift_src
                                .push_str(&format!("if {param}.tag == {i} {{\n"));
                            let ty = self.interface.get_ty(&case.ty);
                            let case_name = format!("F{i}");
                            uwriteln!(
                                self.lift_src,
                                "
                                {lift_name}_ptr := (*{ty})(unsafe.Pointer(&{param}.val))
                                {name}{case_name}(*{lift_name}_ptr)"
                            );
                            self.lift_src.push_str("}\n");
                        }
                    }
                    TypeDefKind::Future(_) => todo!("impl future"),
                    TypeDefKind::Stream(_) => todo!("impl stream"),
                    _ => self.lift_src.push_str(""),
                }
            }
            a => {
                let target_name = self.interface.get_ty(a);

                uwriteln!(self.lift_src, "var {lift_name} {target_name}",);
                self.lift_src
                    .push_str(&format!("{lift_name} = {target_name}({param})\n",));
            }
        }
    }

    fn get_c_field_name(&mut self, field: &Field) -> String {
        field.name.to_snake_case()
    }
}

// TODO: don't copy from gen-guest-c
fn c_int_repr(ty: Int) -> &'static str {
    match ty {
        Int::U8 => "uint8_t",
        Int::U16 => "uint16_t",
        Int::U32 => "uint32_t",
        Int::U64 => "uint64_t",
    }
}

fn flags_repr(f: &Flags) -> Int {
    match f.repr() {
        FlagsRepr::U8 => Int::U8,
        FlagsRepr::U16 => Int::U16,
        FlagsRepr::U32(1) => Int::U32,
        FlagsRepr::U32(2) => Int::U64,
        repr => panic!("unimplemented flags {repr:?}"),
    }
}
