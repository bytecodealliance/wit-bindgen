use heck::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use std::mem;
use std::sync::Arc;
use wit_bindgen_core::wit_parser::abi::{
    AbiVariant, Bindgen, Bitcast, Instruction, LiftLower, WasmType,
};
use wit_bindgen_core::{uwrite, uwriteln, wit_parser::*, Direction, Files, Generator, Ns, Source};

#[derive(Default)]
pub struct Go {
    src: Source,
    in_import: bool,
    opts: Opts,
    export_funcs: Vec<(String, String)>,
    funcs: HashMap<String, Vec<Func>>,
    return_pointer_area_size: usize,
    return_pointer_area_align: usize,
    sizes: SizeAlign,
    names: Ns,

    // The set of types that are considered public (aka need to be in the
    // header file) which are anonymous and we're effectively monomorphizing.
    // This is discovered lazily when printing type names.
    public_anonymous_types: BTreeSet<TypeId>,

    // This is similar to `public_anonymous_types` where it's discovered
    // lazily, but the set here are for private types only used in the
    // implementation of functions. These types go in the implementation file,
    // not the header file.
    private_anonymous_types: BTreeSet<TypeId>,

    // Type definitions for the given `TypeId`. This is printed topologically
    // at the end.
    types: HashMap<TypeId, Source>,

    needs_string: bool,
}

impl Go {
    pub fn new() -> Go {
        Go::default()
    }

    fn abi_variant(dir: Direction) -> AbiVariant {
        // This generator uses the obvious direction to ABI variant mapping.
        match dir {
            Direction::Export => AbiVariant::GuestExport,
            Direction::Import => AbiVariant::GuestImport,
        }
    }

    fn get_ty(ty: &Type) -> &str {
        match ty {
            Type::Bool => "bool",
            Type::U8 => "uint8",
            Type::U16 => "uint16",
            Type::U32 => "uint32",
            Type::U64 => "uint64",
            Type::S8 => "int8",
            Type::S16 => "int16",
            Type::S32 => "int32",
            Type::S64 => "int64",
            Type::Float32 => "float32",
            Type::Float64 => "float64",
            Type::Char => "byte",
            Type::String => "string",
            Type::Handle(_) => todo!(),
            Type::Id(_) => todo!(),
        }
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type) {
        self.src.push_str(Self::get_ty(ty));
    }

    fn get_c_ty(ty: &Type) -> &str {
        match ty {
            Type::Bool => "char",
            Type::U8 => "uint8_t",
            Type::U16 => "uint16_t",
            Type::U32 => "uint32_t",
            Type::U64 => "uint64_t",
            Type::S8 => "int8_t",
            Type::S16 => "int16_t",
            Type::S32 => "int32_t",
            Type::S64 => "int64_t",
            Type::Float32 => "float",
            Type::Float64 => "double",
            Type::Char => "uint32_t",
            Type::String => todo!(),
            Type::Handle(_) => todo!(),
            Type::Id(_) => todo!(),
        }
    }

    fn print_c_ty(&mut self, iface: &Interface, ty: &Type) {
        self.src.push_str(Self::get_c_ty(ty));
    }

    fn get_func_params(&mut self, iface: &Interface, func: &Function) -> String {
        let mut params = String::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            params.push_str(&name);
            params.push_str(" ");
            params.push_str(Self::get_ty(&param));
        }
        params
    }

    fn get_c_func_params(&mut self, iface: &Interface, func: &Function) -> String {
        let mut params = String::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i > 0 {
                params.push_str(", ");
            }
            params.push_str(&name);
            params.push_str(" C.");
            params.push_str(Self::get_c_ty(&param));
        }
        params
    }

    fn get_func_results(&mut self, iface: &Interface, func: &Function) -> String {
        let mut results = String::new();
        match func.results.len() {
            0 => {}
            1 => {
                results.push_str(Self::get_ty(&func.results.iter_types().next().unwrap()));
            }
            _ => todo!(),
        }
        results
    }

    fn get_c_func_signature(&mut self, iface: &Interface, func: &Function) -> String {
        let name = format!(
            "{}{}",
            iface.name.to_camel_case(),
            func.name.to_camel_case()
        );
        match func.results.len() {
            0 => format!("func {}({})", name, self.get_c_func_params(iface, func),),
            1 => format!(
                "func {}({}) C.{}",
                name,
                self.get_c_func_params(iface, func),
                Self::get_c_ty(&func.results.iter_types().next().unwrap())
            ),
            _ => todo!(),
        }
    }

    fn get_c_func_impl(&mut self, iface: &Interface, func: &Function) -> String {
        let invoke = format!(
            "{}.{}({})",
            &iface.name.to_snake_case(),
            &func.name.to_camel_case(),
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
            0 => invoke.clone(),
            1 => format!(
                "return C.{}({})",
                Self::get_c_ty(&func.results.iter_types().next().unwrap()),
                invoke.clone(),
            ),
            _ => todo!(),
        }
    }
}

impl Generator for Go {
    fn preprocess_one(&mut self, iface: &Interface, dir: Direction) {
        println!("preprocess_one");
        let variant = Self::abi_variant(dir);
        self.sizes.fill(iface);
        self.in_import = variant == AbiVariant::GuestImport;

        // add package
        self.src.push_str("package ");
        self.src.push_str(&iface.name.to_snake_case());
        self.src.push_str("\n\n");

        // import C
        self.src.push_str("// #include \"");
        self.src.push_str(&iface.name.to_kebab_case());
        self.src.push_str(".h\"\n");
        self.src.push_str("import \"C\"\n\n");
    }

    fn type_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        record: &Record,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_flags(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        flags: &Flags,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_tuple(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        flags: &Tuple,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_option(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        payload: &Type,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_union(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        union: &Union,
        docs: &Docs,
    ) {
        todo!()
    }

    fn type_enum(&mut self, iface: &Interface, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        todo!()
    }

    fn type_resource(&mut self, iface: &Interface, ty: ResourceId) {
        todo!()
    }

    fn type_alias(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        todo!()
    }

    fn type_list(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        todo!()
    }

    fn type_builtin(&mut self, iface: &Interface, id: TypeId, name: &str, ty: &Type, docs: &Docs) {
        todo!()
    }

    fn import(&mut self, iface: &Interface, func: &Function) {
        println!("import {:?}", func);

        // FIXME: for now I only care about the freestanding kind of functions.
        // I don't care about the static and methods yet.
        match func.kind {
            FunctionKind::Freestanding => {
                self.src.push_str("func ");
                self.src.push_str(&func.name.to_camel_case());
                self.src.push_str("(");
                let params = self.get_func_params(iface, func);
                self.src.push_str(&params);
                self.src.push_str(")");

                //FIXME: assume only one return value for now
                match func.results.len() {
                    0 => {}
                    1 => {
                        self.src.push_str(" ");
                        self.print_ty(iface, &func.results.iter_types().next().unwrap());
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
                self.src.push_str("(");
                for (i, (name, param)) in func.params.iter().enumerate() {
                    if i > 0 {
                        self.src.push_str(", ");
                    }
                    self.src.push_str("C.");
                    self.print_c_ty(iface, param);
                    self.src.push_str("(");
                    self.src.push_str(&name);
                    self.src.push_str(")");
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
                self.src.push_str("}\n");
            }
            _ => {
                panic!("functions other than freestanding are not supported yet");
            }
        }
    }

    fn export(&mut self, iface: &Interface, func: &Function) {
        println!("export {:?}", func);

        // FIXME: for now I only care about the freestanding kind of functions.
        // I don't care about the static and methods yet.
        match func.kind {
            FunctionKind::Freestanding => {
                let interface_method_decl = format!(
                    "{}({}) {}",
                    func.name.to_camel_case(),
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
                    src.push_str("\n");

                    src.push_str(&self.get_c_func_signature(iface, func));
                    src.push_str(" {\n");
                    src.push_str(&self.get_c_func_impl(iface, func));
                    src.push_str("\n}\n");
                    src
                };
                self.export_funcs.push((interface_method_decl, export_func));
            },
            _ => {
                panic!("functions other than freestanding are not supported yet");
            }
        }
    }

    fn finish_one(&mut self, iface: &Interface, files: &mut Files) {
        println!("finish_one");
        if self.in_import == false {
            let interface_var_name = &iface.name.to_snake_case();
            let interface_name = &iface.name.to_camel_case();

            self.src.push_str(
                format!("var {} {} = nil\n", interface_var_name, interface_name).as_str(),
            );
            self.src.push_str(
                format!(
                    "func Set{}(i {}) {{\n    {} = i\n}}\n",
                    interface_name, interface_name, interface_var_name
                )
                .as_str(),
            );
            self.src
                .push_str(format!("type {} interface {{\n", interface_name).as_str());
            for (interface_func_declaration, _) in &self.export_funcs {
                self.src
                    .push_str(format!("    {}\n", interface_func_declaration).as_str());
            }
            self.src.push_str("}\n");

            for (_, export_func) in &self.export_funcs {
                self.src.push_str(&export_func);
            }
        }

        let mut src = mem::take(&mut self.src);
        files.push(
            &format!("{}.go", iface.name.to_snake_case()),
            src.as_bytes(),
        );
    }

    fn type_result(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        result: &Result_,
        docs: &Docs,
    ) {
        todo!()
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    // ...
}

impl Opts {
    pub fn build(&self) -> Go {
        let mut r = Go::new();
        r.opts = self.clone();
        r
    }
}

struct Func {
    src: Source,
}
