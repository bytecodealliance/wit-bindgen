use std::{mem};

use heck::{ToSnakeCase, ToUpperCamelCase, ToLowerCamelCase, ToKebabCase};
use wit_bindgen_core::{
    wit_parser::{Interface, World, SizeAlign, Function, FunctionKind, Type},
    Files, WorldGenerator, Source, InterfaceGenerator as _,
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
    src: String,
    exports: Vec<Source>,
    export_funcs: Vec<(String, String)>,
}

impl TinyGo {
    fn new() -> Self {
        Self::default()
    }
    fn interface<'a>(&'a mut self, iface: &'a Interface, name: &'a str) -> InterfaceGenerator<'a> {
        let mut sizes = SizeAlign::default();
        sizes.fill(iface);
        InterfaceGenerator {
            src: String::new(),
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

        // gen.add_class();
        let src = mem::take(&mut gen.src);
        self.src.push_str(src.as_str());

    }

    fn export(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// {name}\n"));

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }

        self.export_funcs = gen.export_funcs;

    }

    fn export_default(&mut self, name: &str, iface: &Interface, _files: &mut Files) {
        self.src.push_str(&format!("// {name}\n"));

        let mut gen = self.interface(iface, name);
        gen.types();

        for func in iface.functions.iter() {
            gen.export(iface, func);
        }
        self.export_funcs = gen.export_funcs;
    }

    fn finish(&mut self, world: &World, files: &mut Files) {
        if !self.export_funcs.is_empty() {
            let interface_var_name = &world.name.to_snake_case();
            let interface_name = &world.name.to_upper_camel_case();

            self.src.push_str(
                format!("var {interface_var_name} {interface_name} = nil\n").as_str(),
            );
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
                    .push_str(format!("    {interface_func_declaration}\n").as_str());
            }
            self.src.push_str("}\n");

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(export_func);
            }
        }
        

        let src = mem::take(&mut self.src);
        files.push(
            &format!("{}.go", world.name.to_kebab_case()),
            src.as_bytes(),
        );
    }
}

struct InterfaceGenerator<'a> {
    src: String,
    gen: &'a mut TinyGo,
    iface: &'a Interface,
    name: &'a str,
    export_funcs: Vec<(String, String)>,
}

impl InterfaceGenerator<'_> {
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
                        name.to_upper_camel_case()
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
            Type::String => { todo!() }
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
            params.push_str(" C.");
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
            iface.name.to_lower_camel_case(),
            func.name.to_lower_camel_case()
        );
        match func.results.len() {
            0 => format!("func {}({})", name, self.get_c_func_params(iface, func),),
            1 => format!(
                "func {}({}) C.{}",
                name,
                self.get_c_func_params(iface, func),
                self.get_c_ty(func.results.iter_types().next().unwrap())
            ),
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

    fn import(&mut self, iface: &Interface, func: &Function) {
        match func.kind {
            FunctionKind::Freestanding => {
                self.src.push_str("func ");
                self.src.push_str(&func.name.to_lower_camel_case());
                self.src.push('(');
                let params = self.get_func_params(iface, func);
                self.src.push_str(&params);
                self.src.push(')');

                //FIXME: assume only one return value for now
                match func.results.len() {
                    0 => {}
                    1 => {
                        self.src.push(' ');
                        self.print_ty(iface, func.results.iter_types().next().unwrap());
                    }
                    _ => todo!(),
                }

                self.src.push_str(" {\n    ");
                if func.results.len() > 0 {
                    self.src.push_str("res := ");
                }
                let c_name = format!(
                    "{}_{}",
                    iface.name.to_snake_case(),
                    func.name.to_snake_case()
                );
                self.src.push_str("C.");
                self.src.push_str(&c_name);
                self.src.push('(');
                for (i, (name, param)) in func.params.iter().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.src.push_str("C.");
                    self.print_c_ty(iface, param);
                    self.src.push('(');
                    self.src.push_str(name);
                    self.src.push(')');
                }
                self.src.push_str(")\n");

                // return
                match func.results.len() {
                    0 => {}
                    1 => {
                        self.src.push_str("return ");
                        let ty = self.get_func_results(iface, func);
                        self.src.push_str(&ty);
                        self.src.push_str("(res)\n");
                    }
                    _ => todo!(),
                };
                self.src.push_str("}\n\n");
            }
            _ => {
                panic!("functions other than freestanding are not supported yet");
            }
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        println!("export {func:?}");

        // FIXME: for now I only care about the freestanding kind of functions.
        // I don't care about the static and methods yet.
        match func.kind {
            FunctionKind::Freestanding => {
                let interface_method_decl = format!(
                    "{}({}) {}",
                    func.name.to_lower_camel_case(),
                    self.get_func_params(iface, func),
                    self.get_func_results(iface, func)
                );
                let export_func = {
                    let mut src = String::new();
                    src.push_str("//export ");
                    let name = format!(
                        "{}_{}",
                        iface.name.to_snake_case(),
                        func.name.to_snake_case()
                    );
                    src.push_str(&name);
                    src.push('\n');

                    src.push_str(&self.get_c_func_signature(iface, func));
                    src.push_str(" {\n");
                    src.push_str(&self.get_c_func_impl(iface, func));
                    src.push_str("\n}\n");
                    src
                };
                self.export_funcs.push((interface_method_decl, export_func));
            }
            _ => {
                panic!("functions other than freestanding are not supported yet");
            }
        }
    }
}

impl<'a> wit_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, name: &str, record: &wit_bindgen_core::wit_parser::Record, _docs: &wit_bindgen_core::wit_parser::Docs) {
        let name = name.to_uppercase();
        self.src.push_str(&format!(
            "type {name} struct {{\n",
        ));
        for field in record.fields.iter() {
            let ty = self.get_ty(&field.ty);
            self.src.push_str(&format!(
                "   {name} {ty}\n",
                name = field.name.to_upper_camel_case(),
                ty = ty,
            ));
        }
        self.src.push_str("}\n\n");
    }

    fn type_flags(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, name: &str, flags: &wit_bindgen_core::wit_parser::Flags, _docs: &wit_bindgen_core::wit_parser::Docs) {
        let name = name.to_uppercase();
        self.src.push_str(&format!(
            "type {name} uint8\n"
        ));
        self.src.push_str("const (\n");
        for (i, flag) in flags.flags.iter().enumerate() {
            if i == 0 {
                self.src.push_str(&format!(
                    "   {name}_{flag} {name} = 1 << iota\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            }
            else {
                self.src.push_str(&format!(
                    "   {name}_{flag}\n",
                    name = name,
                    flag = flag.name.to_uppercase(),
                ));
            }
        }
        self.src.push_str(")\n\n");
    }

    fn type_tuple(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _flags: &wit_bindgen_core::wit_parser::Tuple, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_variant(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _variant: &wit_bindgen_core::wit_parser::Variant, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_option(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _payload: &wit_bindgen_core::wit_parser::Type, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_result(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _result: &wit_bindgen_core::wit_parser::Result_, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_union(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _union: &wit_bindgen_core::wit_parser::Union, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_enum(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _enum_: &wit_bindgen_core::wit_parser::Enum, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_alias(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _ty: &wit_bindgen_core::wit_parser::Type, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_list(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _ty: &wit_bindgen_core::wit_parser::Type, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }

    fn type_builtin(&mut self, _id: wit_bindgen_core::wit_parser::TypeId, _name: &str, _ty: &wit_bindgen_core::wit_parser::Type, _docs: &wit_bindgen_core::wit_parser::Docs) {
        todo!()
    }
}