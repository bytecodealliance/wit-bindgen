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
        if cfg!(debug_assertions) {
            println!("process id: {}", std::process::id());
            std::thread::sleep(std::time::Duration::from_secs(8));
        }
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
            private_anonymous_types: BTreeSet::new(),
        }
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

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);

        if !self.export_funcs.is_empty() {
            let interface_var_name = &iface.name.to_snake_case();
            let interface_name = &iface.name.to_upper_camel_case();

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

        let mut iface = iface.clone(); //TODO: remove this clone
        iface.name = String::from(name);
        let iface = &iface;

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }

        gen.finish();

        let src = mem::take(&mut gen.src);
        self.src.push_str(&src);

        if !self.export_funcs.is_empty() {
            let interface_var_name = &iface.name.to_snake_case();
            let interface_name = &iface.name.to_upper_camel_case();

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
        let world_name = self.world.clone();
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
                "package {world_name}

                type OptionKind int
                
                const (
                    None OptionKind = iota
                    Some
                )
                
                type Option[T any] struct {{
                    Kind OptionKind
                    Val  T
                }}
                
                func (o Option[T]) IsNone() bool {{
                    return o.Kind == None
                }}
                
                func (o Option[T]) IsSome() bool {{
                    return o.Kind == Some
                }}
                
                func (o Option[T]) Unwrap() T {{
                    if o.Kind != Some {{
                        panic(\"Option is None\")
                    }}
                    return o.Val
                }}
                
                func (o *Option[T]) Set(val T) T {{
                    o.Kind = Some
                    o.Val = val
                    return val
                }}
                
                func (o *Option[T]) Unset() {{
                    o.Kind = None
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
                }}"
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
    private_anonymous_types: BTreeSet<TypeId>,
}

impl InterfaceGenerator<'_> {
    fn get_typedef_target(&mut self, name: &str) -> String {
        format!(
            "{}{}",
            self.iface.name.to_upper_camel_case(),
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
            Type::Char => "uint32".into(),
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
                            self.private_anonymous_types.remove(id);
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
            Type::Bool => "char".into(),
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
                        self.public_anonymous_types.insert(*id);
                        self.private_anonymous_types.remove(id);
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
                    TypeDefKind::Option(_ty) => {
                        todo!()
                    }
                    TypeDefKind::Result(_r) => {
                        todo!()
                    }
                    TypeDefKind::List(_t) => {
                        todo!()
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

    fn get_c_func_params(&mut self, _iface: &Interface, func: &Function) -> String {
        let mut params = String::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            params.push_str(name);
            if self.is_arg_by_pointer(param) {
                params.push_str(" *C.");
            } else {
                params.push_str(" C.");
            }

            // flatten optional types
            let s = self
                .extract_option_ty(param)
                .as_ref()
                .map(|o| self.get_c_ty(o))
                .unwrap_or(self.get_c_ty(param));
            params.push_str(&s);
        }
        params
    }

    fn get_func_results(&mut self, _iface: &Interface, func: &Function) -> String {
        let mut results = String::new();
        match func.results.len() {
            0 => {}
            1 => {
                results.push_str(&self.get_ty(func.results.iter_types().next().unwrap()));
            }
            _ => todo!(),
        }
        results
    }

    fn get_c_func_signature(&mut self, iface: &Interface, func: &Function) -> String {
        let name = format!(
            "{}{}",
            iface.name.to_upper_camel_case(),
            func.name.to_upper_camel_case()
        );
        match func.results.len() {
            0 => format!("func {}({})", name, self.get_c_func_params(iface, func),),
            1 => {
                let return_ty = func.results.iter_types().next().unwrap();
                if self.is_arg_by_pointer(return_ty) {
                    // flatten result types into two arguments as an optimization in C
                    // please see this PR for more details https://github.com/bytecodealliance/wit-bindgen/pull/450
                    // TODO: deduplicate
                    let ret = match self.extract_result_ty(return_ty) {
                        (None, None) => {
                            if let Some(o) = self.extract_option_ty(return_ty) {
                                format!("ret *C.{}) bool", self.get_c_ty(&o))
                            } else {
                                format!("ret *C.{})", self.get_c_ty(return_ty))
                            }
                        }
                        (None, Some(err)) => format!("err *C.{}) bool", self.get_c_ty(&err)),
                        (Some(ok), None) => format!("ret *C.{}) bool", self.get_c_ty(&ok)),
                        (Some(ok), Some(err)) => format!(
                            "ret *C.{}, err *C.{}) bool",
                            self.get_c_ty(&ok),
                            self.get_c_ty(&err)
                        ),
                    };
                    format!(
                        "func {}({}, {}",
                        name,
                        self.get_c_func_params(iface, func),
                        ret,
                    )
                } else {
                    format!(
                        "func {}({}) C.{}",
                        name,
                        self.get_c_func_params(iface, func),
                        self.get_c_ty(return_ty)
                    )
                }
            }
            _ => todo!(),
        }
    }

    fn get_func_signature(&mut self, iface: &Interface, func: &Function) -> String {
        format!(
            "{}{}",
            iface.name.to_upper_camel_case(),
            self.get_func_signature_no_interface(iface, func)
        )
    }

    fn get_func_signature_no_interface(&mut self, iface: &Interface, func: &Function) -> String {
        format!(
            "{}({}) {}",
            func.name.to_upper_camel_case(),
            self.get_func_params(iface, func),
            self.get_func_results(iface, func)
        )
    }

    fn print_func_signature(&mut self, iface: &Interface, func: &Function) {
        let sig = self.get_func_signature(iface, func);
        self.src.push_str(&format!("func {sig} {{\n"));
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

    fn no_default_value(&self, ty: &Type) -> bool {
        match ty {
            Type::Id(id) => match &self.iface.types[*id].kind {
                TypeDefKind::Type(t) => self.no_default_value(t),
                TypeDefKind::List(_) => true,
                TypeDefKind::Option(_) => true,
                TypeDefKind::Result(_) => true,
                TypeDefKind::Future(_) => todo!("is_arg_by_pointer for future"),
                TypeDefKind::Stream(_) => todo!("is_arg_by_pointer for stream"),
                _ => false,
            },
            Type::String => true,
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
                    self.iface().name.to_upper_camel_case(),
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

    fn import(&mut self, iface: &Interface, func: &Function) {
        let mut func_bindgen = FunctionBindgen::new(self, func);
        // lower params to c
        func.params.iter().for_each(|(name, ty)| {
            func_bindgen.lower(name, ty);
        });
        // lift results from c
        match func.results.len() {
            0 => {}
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lift("result", ty, false);
            }
            _ => {
                todo!("does not support multi-results")
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
        c_args: Vec<String>,
        lift_src: &str,
        ret: Vec<String>,
    ) {
        // invoke
        let invoke = format!(
            "C.{}_{}({})",
            iface.name.to_snake_case(),
            func.name.to_snake_case(),
            c_args
                .iter()
                .enumerate()
                .map(|(i, name)| format!(
                    "{}{}",
                    name,
                    if i < func.params.len() - 1 { ", " } else { "" }
                ))
                .collect::<String>()
        );
        match func.results.len() {
            0 => {
                self.src.push_str(&format!("{invoke}\n"));
            }
            1 => {
                let return_ty = func.results.iter_types().next().unwrap();
                if self.is_arg_by_pointer(return_ty) {
                    let mut result_bool = "";
                    let result = if !self.is_result_ty(return_ty) {
                        let optional_type = self.extract_option_ty(return_ty);
                        // flatten optional type or use return type #https://github.com/bytecodealliance/wit-bindgen/pull/453
                        // TODO: reuse from C
                        let c_ret_type = optional_type
                            .as_ref()
                            .map(|o| self.get_c_ty(o))
                            .unwrap_or_else(|| self.get_c_ty(return_ty));
                        self.src.push_str(&format!("var result C.{c_ret_type}\n"));
                        "&result".to_string()
                    } else {
                        result_bool = "result_bool :=";
                        let mut result = String::new();
                        let (ok, err) = self.extract_result_ty(return_ty);
                        if let Some(ok) = ok {
                            let c_ret_type = self.get_c_ty(&ok);
                            self.src.push_str(&format!("var result C.{c_ret_type}\n"));
                            result.push_str("&result")
                        }
                        if let Some(err) = err {
                            let c_ret_type = self.get_c_ty(&err);
                            self.src.push_str(&format!("var err C.{c_ret_type}\n"));
                            if result.is_empty() {
                                result.push_str("&err")
                            } else {
                                result.push_str(", &err")
                            }
                        }
                        result
                    };
                    let invoke = format!(
                        "{result_bool} C.{}_{}({}, {})\n",
                        iface.name.to_snake_case(),
                        func.name.to_snake_case(),
                        c_args
                            .iter()
                            .enumerate()
                            .map(|(i, name)| format!(
                                "&{}{}",
                                name,
                                if i < func.params.len() - 1 { ", " } else { "" }
                            ))
                            .collect::<String>(),
                        result
                    );
                    self.src.push_str(&invoke);
                } else {
                    self.src.push_str(&format!("result := {invoke}\n"));
                }
                self.src.push_str(lift_src);
                self.src.push_str(&format!("return {ret}\n", ret = ret[0]));
            }
            _ => todo!("does not support multi-results"),
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        println!("export {func:?}");

        let mut func_bindgen = FunctionBindgen::new(self, func);
        // lift params to go
        func.params.iter().for_each(|(name, ty)| {
            func_bindgen.lift(name, ty, true);
        });
        // lower result to c
        match func.results.len() {
            0 => {}
            1 => {
                let ty = func.results.iter_types().next().unwrap();
                func_bindgen.lower("result", ty);
            }
            _ => {
                todo!("does not support multi-results")
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
                iface.name.to_snake_case(),
                func.name.to_snake_case()
            );
            src.push_str(&name);
            src.push('\n');

            // signature
            src.push_str(&self.get_c_func_signature(iface, func));
            src.push_str(" {\n");

            // src.push_str(&self.get_c_func_impl(iface, func));
            // prepare args

            src.push_str(&lift_src);

            // invoke
            let invoke = format!(
                "{}.{}({})",
                &iface.name.to_snake_case(),
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
                        match self.extract_result_ty(return_ty) {
                            (None, None) => {
                                src.push_str(&format!("*ret = {lower_result}\n"));
                            }
                            (None, Some(_)) => {
                                uwriteln!(
                                    src,
                                    "
                                    if {lower_result}.is_err {{
                                        *err = {lower_result}.val.err
                                    }}
                                    return result.IsOk()
                                    "
                                );
                            }
                            (Some(_), None) => {
                                uwriteln!(
                                    src,
                                    "
                                    if {lower_result}.is_err == false {{
                                        *ret = {lower_result}.val.ok
                                    }}
                                    return result.IsOk()
                                    "
                                )
                            }
                            (Some(_), Some(_)) => {
                                uwriteln!(
                                    src,
                                    "
                                    if {lower_result}.is_err {{
                                        *err = {lower_result}.val.err
                                    }} else {{
                                        *ret = {lower_result}.val.ok
                                    }}
                                    return result.IsOk()
                                    "
                                );
                            }
                        };
                        if let Some(_o) = self.extract_option_ty(return_ty) {
                            uwriteln!(
                                src,
                                "
                                return result.IsSome()"
                            )
                        }
                    } else {
                        src.push_str(&format!("return {ret}\n", ret = &ret[0]));
                    }
                }
                _ => todo!("does not support multi-results"),
            };

            src.push_str("\n}\n");
            src
        };
        self.gen
            .export_funcs
            .push((interface_method_decl, export_func));
    }

    fn finish(&mut self) {
        while !self.public_anonymous_types.is_empty() {
            for ty in mem::take(&mut self.public_anonymous_types) {
                self.print_anonymous_type(ty);
            }
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
            self.iface().name.to_upper_camel_case(),
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
            self.iface().name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        self.src.push_str(&format!("type {name} uint8\n"));
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
        _id: wit_bindgen_core::wit_parser::TypeId,
        name: &str,
        tuple: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let name = format!(
            "{}{}",
            self.iface().name.to_upper_camel_case(),
            name.to_upper_camel_case()
        );
        self.src.push_str(&format!("type {name} struct {{\n",));
        for (i, ty) in tuple.types.iter().enumerate() {
            let ty = self.get_ty(ty);
            self.src.push_str(&format!("   F{i} {ty}\n",));
        }
        self.src.push_str("}\n\n");
    }

    fn type_variant(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _variant: &wit_bindgen_core::wit_parser::Variant,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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
        _name: &str,
        _union: &wit_bindgen_core::wit_parser::Union,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_enum(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _enum_: &wit_bindgen_core::wit_parser::Enum,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_alias(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
        // let name = format!(
        //     "{}{}",
        //     self.iface().name.to_upper_camel_case(),
        //     name.to_upper_camel_case()
        // );
        // let ty = self.get_ty(ty);
        // self.src.push_str(&format!("type {name} = {ty}\n"));
    }

    fn type_list(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        let ty = self.get_ty(ty);
        self.src.push_str(&ty);
    }

    fn type_builtin(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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

    fn lower(&mut self, name: &str, ty: &Type) {
        let lower_name = format!("lower_{name}");
        if self.interface.extract_option_ty(ty).is_some() {
            self.lower_value(name, ty, lower_name.as_ref(), true);
        } else {
            self.lower_value(name, ty, lower_name.as_ref(), false);
        }
        self.c_args.push(lower_name);
    }

    fn lower_value(&mut self, param: &str, ty: &Type, lower_name: &str, flatten: bool) {
        match ty {
            Type::Bool => self.lower_src.push_str("nil"),
            Type::Char => self.lower_src.push_str("nil"),
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
                            );

                            uwrite!(
                                self.lower_src,
                                "
                                {lower_name}.f{i} = {lower_name}_f{i}
                                "
                            );
                        }
                    }
                    // TypeDefKind::Variant(_) => todo!(),
                    // TypeDefKind::Enum(_) => todo!(),
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
                        );
                        if self.interface.is_not_option(o) && flatten {
                            self.lower_src
                                .push_str(&format!("{lower_name} = {lower_name}_val\n"));
                        } else {
                            self.lower_src
                                .push_str(&format!("{lower_name}.val = {lower_name}_val\n"));
                            self.lower_src
                                .push_str(&format!("{lower_name}.is_some = true\n"));
                        }
                        self.lower_src.push_str("}\n");
                    }
                    TypeDefKind::Result(r) => {
                        let c_typedef_target = self.interface.get_c_ty(&Type::Id(*id));
                        self.interface.gen.needs_import_unsafe = true;

                        self.lower_src
                            .push_str(&format!("var {lower_name} C.{c_typedef_target}\n"));
                        match (r.ok, r.err) {
                            (None, Some(err)) => {
                                let err = self.interface.get_ty(&err);
                                uwrite!(
                                    self.lower_src,
                                    "
                                    {lower_name}.is_err = {param}.IsErr()
                                    {lower_name}_ptr := (*{err})(unsafe.Pointer(&{lower_name}.val))
                                    if {param}.IsErr() {{
                                        *{lower_name}_ptr = {param}.UnwrapErr()
                                    }}
                                    "
                                );
                            }
                            (Some(ok), None) => {
                                let ok = self.interface.get_ty(&ok);
                                uwrite!(
                                    self.lower_src,
                                    "
                                    {lower_name}.is_err = {param}.IsErr()
                                    {lower_name}_ptr := (*{ok})(unsafe.Pointer(&{lower_name}.val))
                                    if {param}.IsOk() {{
                                        *{lower_name}_ptr = {param}.Unwrap()
                                    }}
                                    "
                                );
                            }
                            (Some(ok), Some(err)) => {
                                let ok = self.interface.get_ty(&ok);
                                let err = self.interface.get_ty(&err);
                                uwrite!(
                                    self.lower_src,
                                    "
                                    {lower_name}.is_err = {param}.IsErr()
                                    if {param}.IsOk() {{
                                        {lower_name}_ptr := (*{ok})(unsafe.Pointer(&{lower_name}.val))
                                        *{lower_name}_ptr = {param}.Unwrap()
                                    }} else {{
                                        {lower_name}_ptr := (*{err})(unsafe.Pointer(&{lower_name}.val))
                                        *{lower_name}_ptr = {param}.UnwrapErr()
                                    }}
                                    "
                                );
                            }
                            _ => unreachable!("Result must have at least one type"),
                        }
                    }
                    // TypeDefKind::Union(_) => todo!(),
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
                    // TypeDefKind::Future(_) => todo!(),
                    // TypeDefKind::Stream(_) => todo!(),
                    // TypeDefKind::Type(_) => todo!(),
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

    fn lift(&mut self, name: &str, ty: &Type, in_export: bool) {
        let lift_name = format!("lift_{name}");
        if self.interface.extract_option_ty(ty).is_some() {
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
            Type::Bool => self.lift_src.push_str("nil"),
            Type::Char => self.lift_src.push_str("nil"),
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
                    // TypeDefKind::Variant(_) => todo!(),
                    // TypeDefKind::Enum(_) => todo!(),
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
                            let param = if !in_export {
                                if is_pointer {
                                    let c_target_name = self.interface.get_c_ty(o);
                                    self.lift_src.push_str(&format!(
                                        "var {lift_name}_c C.{c_target_name}\n"
                                    ));
                                    self.lift_src
                                        .push_str(&format!("if {param} == {lift_name}_c {{\n"));
                                } else {
                                    self.lift_src.push_str(&format!("if {param} == 0 {{\n"));
                                }
                                param.to_string()
                            } else {
                                let need_pointer = (count == 0) || count > 0 && is_pointer;
                                if need_pointer {
                                    self.lift_src.push_str(&format!("if {param} == nil {{\n"));
                                } else {
                                    self.lift_src.push_str(&format!("if {param} == 0 {{\n"));
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
                                if in_export {
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{err})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                } else {
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{err})(unsafe.Pointer(&err))\n"
                                    ));
                                    self.lift_src.push_str(&format!("if !result_bool {{ \n"));
                                }
                                self.lift_src
                                    .push_str(&format!("{lift_name}.SetErr(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str(&format!("}} else {{\n"));
                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(struct{{}}{{}})\n"));
                                self.lift_src.push_str(&format!("}}\n"));
                            }
                            (Some(ok), None) => {
                                let ok = self.interface.get_ty(&ok);
                                if in_export {
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{ok})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                } else {
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{ok})(unsafe.Pointer(&result))\n"
                                    ));
                                    self.lift_src.push_str(&format!("if !result_bool {{ \n"));
                                }
                                self.lift_src
                                    .push_str(&format!("{lift_name}.SetErr(struct{{}}{{}})\n"));
                                self.lift_src.push_str(&format!("}} else {{\n"));
                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str(&format!("}}\n"));
                            }
                            (Some(ok), Some(err)) => {
                                let ok = self.interface.get_ty(&ok);
                                let err = self.interface.get_ty(&err);

                                if in_export {
                                    self.lift_src.push_str(&format!("if {param}.is_err {{ \n"));
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{err})(unsafe.Pointer(&{param}.val))\n"));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}.SetErr(*{lift_name}_ptr)\n"
                                    ));
                                    self.lift_src.push_str(&format!("}} else {{\n"));
                                    self.lift_src.push_str(&format!("{lift_name}_ptr := (*{ok})(unsafe.Pointer(&{param}.val))\n"));
                                } else {
                                    self.lift_src.push_str(&format!("if !result_bool {{ \n"));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{err})(unsafe.Pointer(&err))\n"
                                    ));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}.SetErr(*{lift_name}_ptr)\n"
                                    ));
                                    self.lift_src.push_str(&format!("}} else {{\n"));
                                    self.lift_src.push_str(&format!(
                                        "{lift_name}_ptr := (*{ok})(unsafe.Pointer(&result))\n"
                                    ));
                                }

                                self.lift_src
                                    .push_str(&format!("{lift_name}.Set(*{lift_name}_ptr)\n"));
                                self.lift_src.push_str(&format!("}}\n"));
                            }
                            _ => unreachable!("Result must have at least one type"),
                        }
                    }
                    // TypeDefKind::Union(_) => todo!(),
                    TypeDefKind::List(l) => {
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
                    // TypeDefKind::Future(_) => todo!(),
                    // TypeDefKind::Stream(_) => todo!(),
                    // TypeDefKind::Type(_) => todo!(),
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
