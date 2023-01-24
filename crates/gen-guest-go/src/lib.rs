use std::mem;

use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToUpperCamelCase};
use wit_bindgen_core::{
    wit_parser::{
        Field, Flags, FlagsRepr, Function, Int, Interface, SizeAlign, Type, TypeDefKind, World,
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
        }
    }
}

impl WorldGenerator for TinyGo {
    fn preprocess(&mut self, name: &str) {
        // add package
        self.src.push_str("package ");
        self.src.push_str(name.to_snake_case().as_str());
        self.src.push_str("\n\n");

        // import C

        self.src.push_str("// #include \"");
        self.src.push_str(name.to_snake_case().as_str());
        self.src.push_str(".h\"\n");
        self.src.push_str("import \"C\"\n\n");
    }

    fn import(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// {name}\n"));

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.import(iface, func);
        }

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
        let src = mem::take(&mut self.src);
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            src.as_bytes(),
        );
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut TinyGo,
    iface: &'a Interface,
    name: &'a str,
    export_funcs: Vec<(String, String)>,
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
                    wit_bindgen_core::wit_parser::TypeDefKind::Tuple(_) => todo!(),
                    wit_bindgen_core::wit_parser::TypeDefKind::Option(_) => todo!(),
                    wit_bindgen_core::wit_parser::TypeDefKind::Result(_) => todo!(),
                    _ => {
                        if let Some(name) = &ty.name {
                            self.get_typedef_target(name)
                        } else {
                            unreachable!()
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
                todo!()
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
                    None => unreachable!(),
                }
            }
        }
    }

    fn print_c_ty(&mut self, _iface: &Interface, ty: &Type) {
        let ty = self.get_c_ty(ty);
        self.src.push_str(&ty);
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
            match param {
                Type::Id(id) => {
                    let ty = &self.iface.types[*id];
                    match &ty.kind {
                        TypeDefKind::Record(_r) => {
                            params.push_str(" *C.");
                        }
                        _ => {
                            params.push_str(" C.");
                        }
                    }
                }
                _ => {
                    params.push_str(" C.");
                }
            }

            params.push_str(&self.get_c_ty(param));
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
                let sig = format!(
                    "func {}({}) C.{}",
                    name,
                    self.get_c_func_params(iface, func),
                    self.get_c_ty(func.results.iter_types().next().unwrap())
                );
                let result = func.results.iter_types().next().unwrap();
                match result {
                    Type::Id(id) => {
                        let ty = &self.iface.types[*id];
                        match &ty.kind {
                            TypeDefKind::Record(_r) => {
                                format!(
                                    "func {}({}, ret *C.{})",
                                    name,
                                    self.get_c_func_params(iface, func),
                                    self.get_c_ty(func.results.iter_types().next().unwrap())
                                )
                            }
                            _ => sig,
                        }
                    }
                    _ => sig,
                }
            }
            _ => todo!(),
        }
    }

    fn get_c_func_impl(&mut self, iface: &Interface, func: &Function) -> String {
        let invoke = format!(
            "{}.{}({})",
            &iface.name.to_snake_case(),
            &func.name.to_lower_camel_case(),
            func.params
                .iter()
                .enumerate()
                .map(|(i, (name, _))| format!(
                    "{}{}",
                    name,
                    if i < func.params.len() - 1 { ", " } else { "" }
                ))
                .collect::<String>()
        );
        match func.results.len() {
            0 => invoke,
            1 => format!(
                "   return C.{}({})",
                self.get_c_ty(func.results.iter_types().next().unwrap()),
                invoke,
            ),
            _ => todo!(),
        }
    }

    fn get_func_signature(&mut self, iface: &Interface, func: &Function) -> String {
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
                func_bindgen.lift("result", ty);
            }
            _ => {
                todo!("does not support multi-results")
            }
        };
        let c_args = func_bindgen.c_args;
        let ret = func_bindgen.args;

        // // print function signature
        self.print_func_signature(iface, func);

        // body
        // prepare args
        for (_, c_param_decl) in c_args.iter() {
            self.src.push_str(c_param_decl);
        }

        self.import_invoke(iface, func, c_args, ret);

        // return

        self.src.push_str("}\n\n");
    }

    fn import_invoke(
        &mut self,
        iface: &Interface,
        func: &Function,
        c_args: Vec<(String, String)>,
        ret: Vec<(String, String)>,
    ) {
        // invoke
        let invoke = format!(
            "C.{}_{}({})",
            iface.name.to_snake_case(),
            func.name.to_snake_case(),
            c_args
                .iter()
                .enumerate()
                .map(|(i, (name, _))| format!(
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
                let result = func.results.iter_types().next().unwrap();
                match result {
                    Type::Id(id) => {
                        let ty = &self.iface.types[*id];
                        match &ty.kind {
                            TypeDefKind::Record(_r) => {
                                let c_ret_type = self.get_c_ty(result);
                                self.src
                                    .push_str(&format!("result := C.{c_ret_type}{{}}\n"));
                                let invoke = format!(
                                    "C.{}_{}({}, &result)\n",
                                    iface.name.to_snake_case(),
                                    func.name.to_snake_case(),
                                    c_args
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (name, _))| format!(
                                            "&{}{}",
                                            name,
                                            if i < func.params.len() - 1 { ", " } else { "" }
                                        ))
                                        .collect::<String>()
                                );
                                self.src.push_str(&invoke);
                                self.src.push_str(&ret[0].1);
                            }
                            _ => {
                                self.src.push_str(&format!("result := {invoke}\n"));
                                self.src.push_str(&ret[0].1);
                            }
                        }
                    }
                    _ => {
                        self.src.push_str(&format!("result := {invoke}\n"));
                        self.src.push_str(&ret[0].1);
                    }
                }
                self.src
                    .push_str(&format!("return {ret}\n", ret = &ret[0].0));
            }
            _ => todo!("does not support multi-results"),
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        println!("export {func:?}");

        let mut func_bindgen = FunctionBindgen::new(self, func);
        // lift params to go
        func.params.iter().for_each(|(name, ty)| {
            func_bindgen.lift(name, ty);
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

        let interface_method_decl = self.get_func_signature(iface, func);
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
            for (_, c_param_decl) in args.iter() {
                src.push_str(c_param_decl);
            }

            // invoke
            let invoke = format!(
                "{}.{}({})",
                &iface.name.to_snake_case(),
                &func.name.to_upper_camel_case(),
                args.iter()
                    .enumerate()
                    .map(|(i, (name, _))| format!(
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
                    src.push_str(&format!("result := {invoke}\n"));
                    src.push_str(&ret[0].1);
                    let result = func.results.iter_types().next().unwrap();
                    match result {
                        Type::Id(id) => {
                            let ty = &self.iface.types[*id];
                            match &ty.kind {
                                TypeDefKind::Record(_r) => {
                                    src.push_str(&format!("*ret = {ret}\n", ret = &ret[0].0));
                                }
                                _ => {
                                    src.push_str(&format!("return {ret}\n", ret = &ret[0].0));
                                }
                            }
                        }
                        _ => {
                            src.push_str(&format!("return {ret}\n", ret = &ret[0].0));
                        }
                    };
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
        _name: &str,
        _flags: &wit_bindgen_core::wit_parser::Tuple,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _payload: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
    }

    fn type_result(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _result: &wit_bindgen_core::wit_parser::Result_,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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
    }

    fn type_list(
        &mut self,
        _id: wit_bindgen_core::wit_parser::TypeId,
        _name: &str,
        _ty: &wit_bindgen_core::wit_parser::Type,
        _docs: &wit_bindgen_core::wit_parser::Docs,
    ) {
        todo!()
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
    func: &'a Function,
    c_args: Vec<(String, String)>,
    args: Vec<(String, String)>,
}

impl<'a, 'b> FunctionBindgen<'a, 'b> {
    fn new(interface: &'a mut InterfaceGenerator<'b>, func: &'a Function) -> Self {
        Self {
            interface,
            func,
            c_args: Vec::new(),
            args: Vec::new(),
        }
    }

    fn lower(&mut self, name: &str, ty: &Type) {
        let lower_name = format!("lower_{name}");

        let c_arg_decl = format!(
            "   {name} := {value}\n",
            name = lower_name,
            value = self.lower_value(name, ty),
        );
        self.c_args.push((lower_name, c_arg_decl));
    }

    fn lower_value(&mut self, param: &str, ty: &Type) -> String {
        match ty {
            Type::Bool => "nil".into(),
            Type::Char => "nil".into(),
            Type::String => "nil".into(),
            Type::Id(id) => {
                let ty = &self.interface.iface.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let mut src = Source::default();
                        let c_typedef_target = self.get_c_typedef_target(ty.name.as_ref().unwrap()); // okay to unwrap because a record must have a name
                        src.push_str(&format!("C.{c_typedef_target} {{\n"));
                        let f = r
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(_i, field)| {
                                let c_field_name = &self.get_c_field_name(field);
                                let field_name = &self.interface.get_field_name(field);
                                let field_value =
                                    self.lower_value(&format!("{param}.{field_name}"), &field.ty);
                                format!("{}: {}{}", c_field_name, field_value, ",\n")
                            })
                            .collect::<String>();
                        src.push_str(&f);
                        src.push_str("}");
                        src.to_string()
                    }

                    TypeDefKind::Flags(f) => {
                        let int_repr = c_int_repr(flags_repr(f));
                        format!("C.{int_repr}({param})")
                    }
                    // TypeDefKind::Tuple(_) => todo!(),
                    // TypeDefKind::Variant(_) => todo!(),
                    // TypeDefKind::Enum(_) => todo!(),
                    // TypeDefKind::Option(_) => todo!(),
                    // TypeDefKind::Result(_) => todo!(),
                    // TypeDefKind::Union(_) => todo!(),
                    // TypeDefKind::List(_) => todo!(),
                    // TypeDefKind::Future(_) => todo!(),
                    // TypeDefKind::Stream(_) => todo!(),
                    // TypeDefKind::Type(_) => todo!(),
                    _ => "".into(),
                }
            }
            a => {
                format!(
                    "C.{c_type_name}({param_name})",
                    c_type_name = self.interface.get_c_ty(a),
                    param_name = param,
                )
            }
        }
    }

    fn lift(&mut self, name: &str, ty: &Type) {
        let lift_name = format!("lift_{name}");
        let arg_decl = format!(
            "   {name} := {value}\n",
            name = lift_name,
            value = self.lift_value(name, ty),
        );
        self.args.push((lift_name, arg_decl));
    }

    fn lift_value(&mut self, param: &str, ty: &Type) -> String {
        match ty {
            Type::Bool => "nil".into(),
            Type::Char => "nil".into(),
            Type::String => "nil".into(),
            Type::Id(id) => {
                let ty = &self.interface.iface.types[*id]; // receive type

                match &ty.kind {
                    TypeDefKind::Record(r) => {
                        let mut src = Source::default();
                        let typedef_target =
                            self.interface.get_typedef_target(ty.name.as_ref().unwrap());
                        src.push_str(&format!("{typedef_target} {{\n"));
                        let f = r
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(_i, field)| {
                                let field_name = &self.interface.get_field_name(field);
                                let c_field_name = &self.get_c_field_name(field);
                                let field_value =
                                    self.lift_value(&format!("{param}.{c_field_name}"), &field.ty);
                                format!("{}: {}{}", field_name, field_value, ",\n")
                            })
                            .collect::<String>();
                        src.push_str(&f);
                        src.push_str("}");
                        src.to_string()
                    }
                    TypeDefKind::Flags(_f) => {
                        let field = self.interface.get_typedef_target(ty.name.as_ref().unwrap());
                        format!("{field}({param})")
                    }
                    // TypeDefKind::Tuple(_) => todo!(),
                    // TypeDefKind::Variant(_) => todo!(),
                    // TypeDefKind::Enum(_) => todo!(),
                    // TypeDefKind::Option(_) => todo!(),
                    // TypeDefKind::Result(_) => todo!(),
                    // TypeDefKind::Union(_) => todo!(),
                    // TypeDefKind::List(_) => todo!(),
                    // TypeDefKind::Future(_) => todo!(),
                    // TypeDefKind::Stream(_) => todo!(),
                    // TypeDefKind::Type(_) => todo!(),
                    _ => "".into(),
                }
            }
            a => {
                format!(
                    "{type}({param_name})",
                    type = self.interface.get_ty(a),
                    param_name = param,
                )
            }
        }
    }

    fn get_c_typedef_target(&mut self, name: &str) -> String {
        let _src = String::new();
        let iface_snake = self.interface.iface.name.to_snake_case();
        let snake = name.to_snake_case();
        format!("{iface_snake}_{snake}_t")
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
