use heck::*;
use witx_bindgen_gen_core::{witx::*, TypeInfo};

#[derive(Copy, Clone)]
pub enum TypeMode {
    Owned,
    AllBorrowed(&'static str),
    LeafBorrowed(&'static str),
    Lifetime(&'static str),
}

pub trait TypePrint {
    fn tmp(&mut self) -> usize;
    fn push_str(&mut self, s: &str);
    fn info(&self, ty: &Id) -> TypeInfo;
    fn print_usize(&mut self);
    fn print_pointer(&mut self, const_: bool, ty: &TypeRef);
    fn print_borrowed_slice(&mut self, ty: &TypeRef, lifetime: &'static str);
    fn print_borrowed_str(&mut self, lifetime: &'static str);
    fn is_host(&self) -> bool;

    fn rustdoc(&mut self, docs: &str) {
        if docs.trim().is_empty() {
            return;
        }
        for line in docs.lines() {
            self.push_str("/// ");
            self.push_str(line);
            self.push_str("\n");
        }
    }

    fn rustdoc_params(&mut self, docs: &[InterfaceFuncParam], header: &str) {
        let docs = docs
            .iter()
            .filter(|param| param.docs.trim().len() > 0)
            .collect::<Vec<_>>();
        if docs.len() == 0 {
            return;
        }

        self.push_str("///\n");
        self.push_str("/// ## ");
        self.push_str(header);
        self.push_str("\n");
        self.push_str("///\n");

        for param in docs {
            for (i, line) in param.docs.lines().enumerate() {
                self.push_str("/// ");
                // Currently wasi only has at most one return value, so there's no
                // need to indent it or name it.
                if header != "Return" {
                    if i == 0 {
                        self.push_str("* `");
                        self.push_str(to_rust_ident(param.name.as_str()));
                        self.push_str("` - ");
                    } else {
                        self.push_str("  ");
                    }
                }
                self.push_str(line);
                self.push_str("\n");
            }
        }
    }

    fn print_tref(&mut self, ty: &TypeRef, mode: TypeMode) {
        match ty {
            TypeRef::Name(t) => {
                let info = self.info(&t.name);

                // If we're a borrowed piece of data then this is a parameter
                // and we need a leading `&` out in front. Note that the leading
                // `&` is skipped for lists whose typedefs already have a
                // leading `&`.
                if info.owns_data() {
                    match mode {
                        TypeMode::AllBorrowed(lt) => match &**t.type_() {
                            Type::List(_) => {}
                            _ => {
                                self.push_str("&");
                                if lt != "'_" {
                                    self.push_str(lt);
                                }
                                self.push_str(" ");
                            }
                        },
                        _ => {}
                    }
                }

                if mode.lifetime().is_some() {
                    self.push_str(&info.param_name(&t.name));
                } else {
                    self.push_str(&info.result_name(&t.name));
                }

                // If the type recursively owns data and it's a
                // variant/record/list, then we need to place the lifetime
                // parameter on the type as well.
                if info.owns_data() {
                    match &**t.type_() {
                        Type::Variant(_) | Type::Record(_) | Type::List(_) => {
                            self.print_lifetime_param(mode);
                        }
                        _ => {}
                    }
                }
            }
            TypeRef::Value(v) => match &**v {
                Type::Builtin(t) => self.print_builtin(*t),
                Type::List(t) => self.print_list(t, mode),
                Type::Pointer(t) => self.print_pointer(false, t),
                Type::ConstPointer(t) => self.print_pointer(true, t),
                Type::Variant(v) if v.is_bool() => self.push_str("bool"),
                Type::Variant(v) => match v.as_expected() {
                    Some((ok, err)) => {
                        self.push_str("Result<");
                        match ok {
                            Some(ty) => self.print_tref(ty, mode),
                            None => self.push_str("()"),
                        }
                        self.push_str(",");
                        match err {
                            Some(ty) => self.print_tref(ty, mode),
                            None => self.push_str("()"),
                        }
                        self.push_str(">");
                    }
                    None => match v.as_option() {
                        Some(ty) => {
                            self.push_str("Option<");
                            self.print_tref(ty, mode);
                            self.push_str(">");
                        }
                        None => panic!("unsupported anonymous variant"),
                    },
                },
                Type::Record(r) if r.is_tuple() => {
                    self.push_str("(");
                    for member in r.members.iter() {
                        self.print_tref(&member.tref, mode);
                        self.push_str(",");
                    }
                    self.push_str(")");
                }
                t => panic!("unsupported anonymous type reference: {}", t.kind()),
            },
        }
    }

    fn print_list(&mut self, ty: &TypeRef, mode: TypeMode) {
        match &**ty.type_() {
            Type::Builtin(BuiltinType::Char) => match mode.lifetime() {
                Some(lt) => self.print_borrowed_str(lt),
                None => self.push_str("String"),
            },
            t => match mode {
                TypeMode::AllBorrowed(lt) | TypeMode::Lifetime(lt) => {
                    self.print_borrowed_slice(ty, lt);
                }
                TypeMode::LeafBorrowed(lt) => {
                    if t.all_bits_valid() {
                        self.print_borrowed_slice(ty, lt);
                    } else {
                        self.push_str("Vec<");
                        self.print_tref(ty, mode);
                        self.push_str(">");
                    }
                }
                TypeMode::Owned => {
                    self.push_str("Vec<");
                    self.print_tref(ty, TypeMode::Owned);
                    self.push_str(">");
                }
            },
        }
    }

    fn print_builtin(&mut self, ty: BuiltinType) {
        match ty {
            // A C `char` in Rust we just interpret always as `u8`. It's
            // technically possible to use `std::os::raw::c_char` but that's
            // overkill for the purposes that we'll be using this type for.
            BuiltinType::U8 { lang_c_char: _ } => self.push_str("u8"),
            BuiltinType::U16 => self.push_str("u16"),
            BuiltinType::U32 {
                lang_ptr_size: false,
            } => self.push_str("u32"),
            BuiltinType::U32 {
                lang_ptr_size: true,
            } => self.print_usize(),
            BuiltinType::U64 => self.push_str("u64"),
            BuiltinType::S8 => self.push_str("i8"),
            BuiltinType::S16 => self.push_str("i16"),
            BuiltinType::S32 => self.push_str("i32"),
            BuiltinType::S64 => self.push_str("i64"),
            BuiltinType::F32 => self.push_str("f32"),
            BuiltinType::F64 => self.push_str("f64"),
            BuiltinType::Char => self.push_str("char"),
        }
    }

    fn print_lifetime_param(&mut self, mode: TypeMode) {
        if let Some(lt) = mode.lifetime() {
            self.push_str("<");
            self.push_str(lt);
            self.push_str(">");
        }
    }

    fn int_repr(&mut self, repr: IntRepr) {
        self.push_str(int_repr(repr));
    }

    fn wasm_type(&mut self, ty: WasmType) {
        self.push_str(wasm_type(ty));
    }

    fn modes_of(&self, ty: &Id) -> Vec<(String, TypeMode)> {
        let info = self.info(ty);
        let mut result = Vec::new();
        if info.owns_data() {
            if info.param {
                let mode = if self.is_host() {
                    TypeMode::LeafBorrowed("'a")
                } else {
                    TypeMode::Lifetime("'a")
                };
                result.push((info.param_name(ty), mode));
            }
            if info.result {
                result.push((info.result_name(ty), TypeMode::Owned));
            }
        } else {
            result.push((info.result_name(ty), TypeMode::Owned));
        }
        return result;
    }

    fn print_typedef_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
        let info = self.info(name);
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            if record.is_tuple() {
                self.push_str(&format!("pub type {}", name));
                self.print_lifetime_param(mode);
                self.push_str(" = (");
                for member in record.members.iter() {
                    self.print_tref(&member.tref, mode);
                    self.push_str(",");
                }
                self.push_str(");\n");
            } else {
                if !info.has_handle {
                    if !info.owns_data() {
                        self.push_str("#[repr(C)]\n");
                        self.push_str("#[derive(Copy)]\n");
                    }
                    self.push_str("#[derive(Clone)]\n");
                }
                self.push_str("#[derive(Debug)]\n");
                self.push_str(&format!("pub struct {}\n", name));
                self.print_lifetime_param(mode);
                self.push_str(" {\n");
                for member in record.members.iter() {
                    self.rustdoc(&member.docs);
                    self.push_str("pub ");
                    self.push_str(to_rust_ident(member.name.as_str()));
                    self.push_str(": ");
                    self.print_tref(&member.tref, mode);
                    self.push_str(",\n");
                }
                self.push_str("}\n");
            }
        }
    }

    fn print_typedef_variant(&mut self, name: &Id, variant: &Variant, docs: &str) {
        // TODO: should this perhaps be an attribute in the witx file?
        let is_error = name.as_str().contains("errno") && variant.is_enum();
        let info = self.info(name);

        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            if variant.is_bool() {
                self.push_str(&format!("pub type {} = bool;\n", name));
                continue;
            } else if let Some(ty) = variant.as_option() {
                self.push_str(&format!("pub type {}", name));
                self.print_lifetime_param(mode);
                self.push_str("= Option<");
                self.print_tref(ty, mode);
                self.push_str(">;\n");
                continue;
            } else if let Some((ok, err)) = variant.as_expected() {
                self.push_str(&format!("pub type {}", name));
                self.print_lifetime_param(mode);
                self.push_str("= Result<");
                match ok {
                    Some(ty) => self.print_tref(ty, mode),
                    None => self.push_str("()"),
                }
                self.push_str(",");
                match err {
                    Some(ty) => self.print_tref(ty, mode),
                    None => self.push_str("()"),
                }
                self.push_str(">;\n");
                continue;
            }
            if !info.has_handle {
                if variant.is_enum() {
                    self.push_str("#[repr(");
                    self.int_repr(variant.tag_repr);
                    self.push_str(")]\n#[derive(Copy, PartialEq, Eq)]\n");
                } else if !info.owns_data() {
                    self.push_str("#[derive(Copy)]\n");
                }
                self.push_str("#[derive(Clone)]\n");
            }
            if !is_error {
                self.push_str("#[derive(Debug)]\n");
            }
            self.push_str(&format!("pub enum {}", name.to_camel_case()));
            self.print_lifetime_param(mode);
            self.push_str("{\n");
            for case in variant.cases.iter() {
                self.rustdoc(&case.docs);
                self.push_str(&case_name(&case.name));
                if let Some(ty) = &case.tref {
                    self.push_str("(");
                    self.print_tref(ty, mode);
                    self.push_str(")")
                }
                self.push_str(",\n");
            }
            self.push_str("}\n");

            // Auto-synthesize an implementation of the standard `Error` trait for
            // error-looking types based on their name.
            if is_error {
                self.push_str("impl ");
                self.push_str(&name);
                self.push_str("{\n");

                self.push_str("pub fn name(&self) -> &'static str {\n");
                self.push_str("match self {");
                for case in variant.cases.iter() {
                    self.push_str(&name);
                    self.push_str("::");
                    self.push_str(&case_name(&case.name));
                    self.push_str(" => \"");
                    self.push_str(case.name.as_str());
                    self.push_str("\",");
                }
                self.push_str("}\n");
                self.push_str("}\n");

                self.push_str("pub fn message(&self) -> &'static str {\n");
                self.push_str("match self {");
                for case in variant.cases.iter() {
                    self.push_str(&name);
                    self.push_str("::");
                    self.push_str(&case_name(&case.name));
                    self.push_str(" => \"");
                    self.push_str(case.docs.trim());
                    self.push_str("\",");
                }
                self.push_str("}\n");
                self.push_str("}\n");

                self.push_str("}\n");

                self.push_str("impl std::fmt::Debug for ");
                self.push_str(&name);
                self.push_str(
                    "{\nfn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n",
                );
                self.push_str("f.debug_struct(\"");
                self.push_str(&name);
                self.push_str("\")");
                self.push_str(".field(\"code\", &(*self as i32))");
                self.push_str(".field(\"name\", &self.name())");
                self.push_str(".field(\"message\", &self.message())");
                self.push_str(".finish()");
                self.push_str("}\n");
                self.push_str("}\n");

                self.push_str("impl std::fmt::Display for ");
                self.push_str(&name);
                self.push_str(
                    "{\nfn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n",
                );
                self.push_str("write!(f, \"{} (error {})\", self.name(), *self as i32)");
                self.push_str("}\n");
                self.push_str("}\n");
                self.push_str("\n");
                self.push_str("impl std::error::Error for ");
                self.push_str(&name);
                self.push_str("{}\n");
            }
        }
    }

    fn print_typedef_alias(&mut self, name: &Id, ty: &NamedType, docs: &str) {
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_lifetime_param(mode);
            self.push_str(" = ");
            let info = self.info(&ty.name);
            match mode.lifetime() {
                Some(_) => self.push_str(&info.param_name(&ty.name)),
                None => self.push_str(&info.result_name(&ty.name)),
            }
            self.print_lifetime_param(mode);
            self.push_str(";\n");
        }
    }

    fn print_type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str) {
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_lifetime_param(mode);
            self.push_str(" = ");
            self.print_list(ty, mode);
            self.push_str(";\n");
        }
    }

    fn record_lower(
        &mut self,
        ty: &RecordDatatype,
        name: Option<&NamedType>,
        tmp: usize,
        operand: &str,
        results: &mut Vec<String>,
    ) {
        if ty.is_tuple() {
            self.push_str("let (");
            for i in 0..ty.members.len() {
                let arg = format!("t{}_{}", tmp, i);
                self.push_str(&arg);
                self.push_str(",");
                results.push(arg);
            }
            self.push_str(") = ");
            self.push_str(operand);
            self.push_str(";\n");
        } else if let Some(name) = name {
            let info = self.info(&name.name);
            self.push_str("let ");
            if self.is_host() {
                self.push_str(&info.result_name(&name.name));
            } else {
                self.push_str(&info.param_name(&name.name));
            }
            self.push_str("{ ");
            for member in ty.members.iter() {
                let arg = format!("{}{}", member.name.as_str(), tmp);
                self.push_str(to_rust_ident(member.name.as_str()));
                self.push_str(":");
                self.push_str(&arg);
                self.push_str(",");
                results.push(arg);
            }
            self.push_str("} = ");
            self.push_str(operand);
            self.push_str(";\n");
        } else {
            unimplemented!()
        }
    }

    fn record_lift(
        &mut self,
        ty: &RecordDatatype,
        name: Option<&NamedType>,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        if ty.is_tuple() {
            if operands.len() == 1 {
                results.push(format!("({},)", operands[0]));
            } else {
                results.push(format!("({})", operands.join(",")));
            }
        } else if let Some(name) = name {
            let info = self.info(&name.name);
            let mut result = if self.is_host() {
                info.param_name(&name.name)
            } else {
                info.result_name(&name.name)
            };
            result.push_str("{");
            for (member, val) in ty.members.iter().zip(operands) {
                result.push_str(to_rust_ident(member.name.as_str()));
                result.push_str(":");
                result.push_str(&val);
                result.push_str(",");
            }
            result.push_str("}");
            results.push(result);
        } else {
            unimplemented!()
        }
    }

    fn let_results(&mut self, amt: usize, results: &mut Vec<String>) {
        match amt {
            0 => {}
            1 => {
                let tmp = self.tmp();
                let res = format!("result{}", tmp);
                self.push_str("let ");
                self.push_str(&res);
                results.push(res);
                self.push_str(" = ");
            }
            n => {
                let tmp = self.tmp();
                self.push_str("let (");
                for i in 0..n {
                    let arg = format!("result{}_{}", tmp, i);
                    self.push_str(&arg);
                    self.push_str(",");
                    results.push(arg);
                }
                self.push_str(") = ");
            }
        }
    }

    fn variant_lower(
        &mut self,
        ty: &Variant,
        name: Option<&NamedType>,
        nresults: usize,
        operand: &str,
        results: &mut Vec<String>,
        blocks: Vec<String>,
    ) {
        // If this is a named enum with no type payloads and we're
        // producing a singular result, then we know we're directly
        // converting from the Rust enum to the integer discriminant. In
        // this scenario we can optimize a bit and use just `as i32`
        // instead of letting LLVM figure out it can do the same with
        // optimizing the `match` generated below.
        if nresults == 1 && name.is_some() && ty.cases.iter().all(|c| c.tref.is_none()) {
            results.push(format!("{} as i32", operand));
            return;
        }

        self.let_results(nresults, results);
        self.push_str("match ");
        self.push_str(operand);
        self.push_str("{\n");
        for (case, block) in ty.cases.iter().zip(blocks) {
            if ty.is_bool() {
                self.push_str(case.name.as_str());
            } else if ty.as_expected().is_some() {
                self.push_str(&case.name.as_str().to_camel_case());
                self.push_str("(");
                self.push_str(if case.tref.is_some() { "e" } else { "()" });
                self.push_str(")");
            } else if ty.as_option().is_some() {
                self.push_str(&case.name.as_str().to_camel_case());
                if case.tref.is_some() {
                    self.push_str("(e)");
                }
            } else if let Some(name) = name {
                self.push_str(&name.name.as_str().to_camel_case());
                self.push_str("::");
                self.push_str(&case_name(&case.name));
                if case.tref.is_some() {
                    self.push_str("(e)");
                }
            } else {
                unimplemented!()
            }
            self.push_str(" => { ");
            self.push_str(&block);
            self.push_str("}\n");
        }
        self.push_str("};\n");
    }

    fn variant_lift_case(
        &mut self,
        ty: &Variant,
        name: Option<&NamedType>,
        case: &Case,
        block: &str,
        result: &mut String,
    ) {
        if ty.is_bool() {
            result.push_str(case.name.as_str());
        } else if ty.as_expected().is_some() {
            result.push_str(&case.name.as_str().to_camel_case());
            result.push_str("(");
            result.push_str(block);
            result.push_str(")");
        } else if ty.as_option().is_some() {
            result.push_str(&case.name.as_str().to_camel_case());
            if case.tref.is_some() {
                result.push_str("(");
                result.push_str(block);
                result.push_str(")");
            }
        } else if let Some(name) = name {
            result.push_str(&name.name.as_str().to_camel_case());
            result.push_str("::");
            result.push_str(&case_name(&case.name));
            if case.tref.is_some() {
                result.push_str("(");
                result.push_str(block);
                result.push_str(")");
            }
        } else {
            unimplemented!()
        }
    }
}

impl TypeMode {
    fn lifetime(&self) -> Option<&'static str> {
        match self {
            TypeMode::Owned => None,
            TypeMode::AllBorrowed(s) | TypeMode::LeafBorrowed(s) | TypeMode::Lifetime(s) => {
                Some(*s)
            }
        }
    }
}

pub fn to_rust_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "type" => "type_",
        "yield" => "yield_",
        s => s,
    }
}

pub fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "i32",
        WasmType::I64 => "i64",
        WasmType::F32 => "f32",
        WasmType::F64 => "f64",
    }
}

pub fn int_repr(repr: IntRepr) -> &'static str {
    match repr {
        IntRepr::U8 => "u8",
        IntRepr::U16 => "u16",
        IntRepr::U32 => "u32",
        IntRepr::U64 => "u64",
    }
}

pub trait TypeInfoExt {
    fn as_type_info(&self) -> &TypeInfo;

    fn owns_data(&self) -> bool {
        let info = self.as_type_info();
        info.has_list || info.has_handle
    }

    fn param_name(&self, name: &Id) -> String {
        let name = name.as_str().to_camel_case();
        if self.as_type_info().result && self.owns_data() {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, name: &Id) -> String {
        let name = name.as_str().to_camel_case();
        if self.as_type_info().param && self.owns_data() {
            format!("{}Result", name)
        } else {
            name
        }
    }
}

impl TypeInfoExt for TypeInfo {
    fn as_type_info(&self) -> &TypeInfo {
        self
    }
}

pub fn case_name(id: &Id) -> String {
    let s = id.as_str();
    if s.chars().next().unwrap().is_alphabetic() {
        s.to_camel_case()
    } else {
        format!("V{}", s)
    }
}

pub fn bitcast(casts: &[Bitcast], operands: &[String], results: &mut Vec<String>) {
    for (cast, operand) in casts.iter().zip(operands) {
        results.push(match cast {
            Bitcast::None => operand.clone(),
            Bitcast::F32ToF64 => format!("f64::from({})", operand),
            Bitcast::I32ToI64 => format!("i64::from({})", operand),
            Bitcast::F32ToI32 => format!("({}).to_bits() as i32", operand),
            Bitcast::F64ToI64 => format!("({}).to_bits() as i64", operand),
            Bitcast::F64ToF32 => format!("{} as f32", operand),
            Bitcast::I64ToI32 => format!("{} as i32", operand),
            Bitcast::I32ToF32 => format!("f32::from_bits({} as u32)", operand),
            Bitcast::I64ToF64 => format!("f64::from_bits({} as u64)", operand),
            Bitcast::F32ToI64 => format!("i64::from(({}).to_bits())", operand),
            Bitcast::I64ToF32 => format!("f32::from_bits({} as u32)", operand),
        });
    }
}
