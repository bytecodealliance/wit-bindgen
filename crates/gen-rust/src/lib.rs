use heck::*;
use witx_bindgen_gen_core::{witx::*, TypeInfo, Types};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeMode {
    Owned,
    AllBorrowed(&'static str),
    LeafBorrowed(&'static str),
    HandlesBorrowed(&'static str),
}

#[derive(Debug, Copy, Clone)]
pub enum Visibility {
    Pub,
    PubSuper,
    Private,
}

pub trait TypePrint {
    fn krate(&self) -> &'static str;
    fn tmp(&mut self) -> usize;
    fn push_str(&mut self, s: &str);
    fn info(&self, ty: &Id) -> TypeInfo;
    fn types_mut(&mut self) -> &mut Types;
    fn print_usize(&mut self);
    fn print_pointer(&mut self, const_: bool, ty: &TypeRef);
    fn print_borrowed_slice(&mut self, mutbl: bool, ty: &TypeRef, lifetime: &'static str);
    fn print_borrowed_str(&mut self, lifetime: &'static str);
    fn call_mode(&self) -> CallMode;
    fn default_param_mode(&self) -> TypeMode;
    fn handle_projection(&self) -> Option<(&'static str, String)>;

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

    fn rustdoc_params(&mut self, docs: &[Param], header: &str) {
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

    fn print_signature(
        &mut self,
        func: &Function,
        visibility: Visibility,
        unsafe_: bool,
        self_arg: bool,
        param_mode: TypeMode,
    ) -> Vec<String> {
        let params = self.print_docs_and_params(func, visibility, unsafe_, self_arg, param_mode);
        if func.results.len() > 0 {
            self.push_str(" -> ");
            self.print_results(func);
        }
        params
    }

    fn print_docs_and_params(
        &mut self,
        func: &Function,
        visibility: Visibility,
        unsafe_: bool,
        self_arg: bool,
        param_mode: TypeMode,
    ) -> Vec<String> {
        let rust_name = func.name.as_ref().to_snake_case();
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        self.rustdoc_params(&func.results, "Return");

        match visibility {
            Visibility::Pub => self.push_str("pub "),
            Visibility::PubSuper => self.push_str("pub(super) "),
            Visibility::Private => (),
        }
        if unsafe_ {
            self.push_str("unsafe ");
        }
        self.push_str("fn ");
        self.push_str(to_rust_ident(&rust_name));

        self.push_str("(");
        if self_arg {
            self.push_str("&self,");
        }
        let mut params = Vec::new();
        for param in func.params.iter() {
            self.push_str(to_rust_ident(param.name.as_str()));
            params.push(to_rust_ident(param.name.as_str()).to_string());
            self.push_str(": ");
            self.print_tref(&param.tref, param_mode);
            self.push_str(",");
        }
        self.push_str(")");
        params
    }

    fn print_results(&mut self, func: &Function) {
        match func.results.len() {
            0 => self.push_str("()"),
            1 => {
                self.print_tref(&func.results[0].tref, TypeMode::Owned);
            }
            _ => {
                self.push_str("(");
                for result in func.results.iter() {
                    self.print_tref(&result.tref, TypeMode::Owned);
                    self.push_str(", ");
                }
                self.push_str(")");
            }
        }
    }

    fn print_tref(&mut self, ty: &TypeRef, mode: TypeMode) {
        match ty {
            TypeRef::Name(t) => {
                let info = self.info(&t.name);
                let lt = self.lifetime_for(&info, mode);
                let ty = &**t.type_();

                match ty {
                    Type::Handle(_) => {
                        // Borrowed handles are always behind a reference since
                        // in that case we never take ownership of the handle.
                        if let Some(lt) = lt {
                            self.push_str("&");
                            if lt != "'_" {
                                self.push_str(lt);
                            }
                            self.push_str(" ");
                        }

                        if let Some((proj, _)) = self.handle_projection() {
                            self.push_str(proj);
                            self.push_str("::");
                        }
                        self.push_str(&t.name.as_str().to_camel_case());
                    }
                    _ => {
                        let name = if lt.is_some() {
                            self.param_name(&t.name)
                        } else {
                            self.result_name(&t.name)
                        };
                        self.push_str(&name);

                        // If the type recursively owns data and it's a
                        // variant/record/list, then we need to place the
                        // lifetime parameter on the type as well.
                        if info.owns_data() {
                            match ty {
                                Type::Variant(_)
                                | Type::Record(_)
                                | Type::List(_)
                                | Type::Buffer(_) => {
                                    self.print_generics(&info, lt, false);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            TypeRef::Value(v) => match &**v {
                Type::Builtin(t) => self.print_builtin(*t),
                Type::List(t) => self.print_list(t, mode),

                Type::Pointer(t) => self.print_pointer(false, t),
                Type::ConstPointer(t) => self.print_pointer(true, t),

                // Variants can be printed natively if they're `Option`,
                // `Result` , or `bool`, otherwise they must be named for now.
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

                // Tuple-like records are mapped directly to Rust tuples of
                // types. Note the trailing comma after each member to
                // appropriately handle 1-tuples.
                Type::Record(r) if r.is_tuple() => {
                    self.push_str("(");
                    for member in r.members.iter() {
                        self.print_tref(&member.tref, mode);
                        self.push_str(",");
                    }
                    self.push_str(")");
                }

                Type::Buffer(r) => self.print_buffer(r, mode),

                Type::Record(_) | Type::Handle(_) => {
                    panic!("unsupported anonymous type reference: {}", v.kind())
                }
            },
        }
    }

    fn print_list(&mut self, ty: &TypeRef, mode: TypeMode) {
        match &**ty.type_() {
            Type::Builtin(BuiltinType::Char) => match mode {
                TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) => {
                    self.print_borrowed_str(lt)
                }
                TypeMode::Owned | TypeMode::HandlesBorrowed(_) => self.push_str("String"),
            },
            t => match mode {
                TypeMode::AllBorrowed(lt) => {
                    let mutbl = self.needs_mutable_slice(ty);
                    self.print_borrowed_slice(mutbl, ty, lt);
                }
                TypeMode::LeafBorrowed(lt) => {
                    if t.all_bits_valid() {
                        let mutbl = self.needs_mutable_slice(ty);
                        self.print_borrowed_slice(mutbl, ty, lt);
                    } else {
                        self.push_str("Vec<");
                        self.print_tref(ty, mode);
                        self.push_str(">");
                    }
                }
                TypeMode::HandlesBorrowed(_) | TypeMode::Owned => {
                    self.push_str("Vec<");
                    self.print_tref(ty, mode);
                    self.push_str(">");
                }
            },
        }
    }

    fn print_buffer(&mut self, b: &Buffer, mode: TypeMode) {
        let lt = match mode {
            TypeMode::AllBorrowed(s) | TypeMode::HandlesBorrowed(s) | TypeMode::LeafBorrowed(s) => {
                s
            }
            TypeMode::Owned => unimplemented!(),
        };
        let prefix = if b.out { "Out" } else { "In" };
        match self.call_mode() {
            // Defined exports means rust-compiled-to-wasm exporting something,
            // and buffers there are all using handles, so they use special types.
            CallMode::DefinedExport => {
                let krate = self.krate();
                self.push_str(krate);
                self.push_str("::exports::");
                self.push_str(prefix);
                self.push_str("Buffer");
                if b.tref.type_().all_bits_valid() {
                    self.push_str("Raw");
                }
                self.push_str("<");
                self.push_str(lt);
                self.push_str(", ");
                self.print_tref(&b.tref, if b.out { TypeMode::Owned } else { mode });
                self.push_str(">");
            }

            // Declared exports means host Rust is calling wasm. If all bits are
            // valid we use raw slices (e.g. u8/u64/etc). Otherwise input
            // buffers (input to wasm) is `ExactSizeIterator` and output buffers
            // (output from wasm) is `&mut Vec`
            CallMode::DeclaredExport => {
                if b.tref.type_().all_bits_valid() {
                    self.print_borrowed_slice(b.out, &b.tref, lt);
                } else if b.out {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                    }
                    self.push_str(" mut Vec<");
                    self.print_tref(&b.tref, if b.out { TypeMode::Owned } else { mode });
                    self.push_str(">");
                } else {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                    }
                    self.push_str(" mut (dyn ExactSizeIterator<Item = ");
                    self.print_tref(&b.tref, if b.out { TypeMode::Owned } else { mode });
                    self.push_str(">");
                    if lt != "'_" {
                        self.push_str(" + ");
                        self.push_str(lt);
                    }
                    self.push_str(")");
                }
            }

            CallMode::DeclaredImport | CallMode::DefinedImport => {
                if b.tref.type_().all_bits_valid() {
                    self.print_borrowed_slice(b.out, &b.tref, lt);
                } else {
                    if let TypeMode::AllBorrowed(_) = mode {
                        self.push_str("&");
                        if lt != "'_" {
                            self.push_str(lt);
                        }
                        self.push_str(" mut ");
                    }
                    let krate = self.krate();
                    self.push_str(krate);
                    self.push_str("::imports::");
                    self.push_str(prefix);
                    self.push_str("Buffer<");
                    self.push_str(lt);
                    self.push_str(", ");
                    self.print_tref(&b.tref, if b.out { TypeMode::Owned } else { mode });
                    self.push_str(">");
                }
            }
        }
    }

    fn print_rust_slice(&mut self, mutbl: bool, ty: &TypeRef, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if mutbl {
            self.push_str(" mut ");
        }
        self.push_str("[");
        self.print_tref(ty, TypeMode::AllBorrowed(lifetime));
        self.push_str("]");
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

    fn print_generics(&mut self, info: &TypeInfo, lifetime: Option<&str>, bound: bool) {
        let proj = if info.has_handle {
            self.handle_projection()
        } else {
            None
        };
        if lifetime.is_none() && proj.is_none() {
            return;
        }
        self.push_str("<");
        if let Some(lt) = lifetime {
            self.push_str(lt);
            self.push_str(",");
        }
        if let Some((proj, trait_bound)) = proj {
            self.push_str(proj);
            if bound {
                self.push_str(": ");
                self.push_str(&trait_bound);
            }
        }
        self.push_str(">");
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
        if info.param {
            result.push((self.param_name(ty), self.default_param_mode()));
        }
        if info.result && (!info.param || self.uses_two_names(&info)) {
            result.push((self.result_name(ty), TypeMode::Owned));
        }
        return result;
    }

    fn print_typedef_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
        let info = self.info(name);
        for (name, mode) in self.modes_of(name) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            if record.is_tuple() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
                self.push_str(" = (");
                for member in record.members.iter() {
                    self.print_tref(&member.tref, mode);
                    self.push_str(",");
                }
                self.push_str(");\n");
            } else {
                if info.has_in_buffer || info.has_out_buffer {
                    // skip copy/clone ...
                } else if lt.is_some() || !info.owns_data() {
                    self.push_str("#[repr(C)]\n");
                    self.push_str("#[derive(Copy, Clone)]\n");
                } else if !info.has_handle {
                    self.push_str("#[derive(Clone)]\n");
                }
                if !info.has_in_buffer {
                    self.push_str("#[derive(Debug)]\n");
                }
                self.push_str(&format!("pub struct {}\n", name));
                self.print_generics(&info, lt, true);
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
            let lt = self.lifetime_for(&info, mode);
            if variant.is_bool() {
                self.push_str(&format!("pub type {} = bool;\n", name));
                continue;
            } else if let Some(ty) = variant.as_option() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
                self.push_str("= Option<");
                self.print_tref(ty, mode);
                self.push_str(">;\n");
                continue;
            } else if let Some((ok, err)) = variant.as_expected() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
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
            if variant.is_enum() {
                self.push_str("#[repr(");
                self.int_repr(variant.tag_repr);
                self.push_str(")]\n#[derive(Clone, Copy, PartialEq, Eq)]\n");
            } else if info.has_in_buffer || info.has_out_buffer {
                // skip copy/clone
            } else if lt.is_some() || !info.owns_data() {
                self.push_str("#[derive(Clone, Copy)]\n");
            }
            if !is_error && !info.has_in_buffer {
                self.push_str("#[derive(Debug)]\n");
            }
            self.push_str(&format!("pub enum {}", name.to_camel_case()));
            self.print_generics(&info, lt, true);
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
        let info = self.info(&ty.name);
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            let lt = self.lifetime_for(&info, mode);
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            let name = match lt {
                Some(_) => self.param_name(&ty.name),
                None => self.result_name(&ty.name),
            };
            self.push_str(&name);
            self.print_generics(&info, lt, false);
            self.push_str(";\n");
        }
    }

    fn print_type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str) {
        let info = self.info(name);
        for (name, mode) in self.modes_of(name) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            self.print_list(ty, mode);
            self.push_str(";\n");
        }
    }

    fn print_typedef_buffer(&mut self, name: &Id, b: &Buffer, docs: &str) {
        let info = self.info(name);
        for (name, mode) in self.modes_of(name) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            self.print_buffer(b, mode);
            self.push_str(";\n");
        }
    }

    fn record_lower(
        &mut self,
        ty: &RecordDatatype,
        name: Option<&NamedType>,
        operand: &str,
        results: &mut Vec<String>,
    ) {
        let tmp = self.tmp();
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
            self.push_str("let ");
            let name = match self.call_mode() {
                // Lowering the result of a defined function means we're
                // lowering the return value.
                CallMode::DefinedImport | CallMode::DefinedExport => self.result_name(&name.name),
                // Lowering the result of a declared function means we're
                // lowering a parameter.
                CallMode::DeclaredImport | CallMode::DeclaredExport => self.param_name(&name.name),
            };
            self.push_str(&name);
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
            let mut result = match self.call_mode() {
                // Lifting to a defined function means that we're lifting into
                // a parameter
                CallMode::DefinedImport | CallMode::DefinedExport => self.param_name(&name.name),
                // Lifting for a declared function means we're lifting one of
                // the return values.
                CallMode::DeclaredImport | CallMode::DeclaredExport => self.result_name(&name.name),
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

    fn param_name(&self, ty: &Id) -> String {
        let info = self.info(ty);
        let name = ty.as_str().to_camel_case();
        if self.uses_two_names(&info) {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, ty: &Id) -> String {
        let info = self.info(ty);
        let name = ty.as_str().to_camel_case();
        if self.uses_two_names(&info) {
            format!("{}Result", name)
        } else {
            name
        }
    }

    fn uses_two_names(&self, info: &TypeInfo) -> bool {
        info.owns_data()
            && info.param
            && info.result
            && match self.default_param_mode() {
                TypeMode::AllBorrowed(_) | TypeMode::LeafBorrowed(_) => true,
                TypeMode::HandlesBorrowed(_) => info.has_handle,
                TypeMode::Owned => false,
            }
    }

    fn lifetime_for(&self, info: &TypeInfo, mode: TypeMode) -> Option<&'static str> {
        match mode {
            TypeMode::AllBorrowed(s) | TypeMode::LeafBorrowed(s)
                if info.has_list
                    || info.has_handle
                    || info.has_out_buffer
                    || info.has_in_buffer =>
            {
                Some(s)
            }
            TypeMode::HandlesBorrowed(s)
                if info.has_handle || info.has_in_buffer || info.has_out_buffer =>
            {
                Some(s)
            }
            _ => None,
        }
    }

    fn needs_mutable_slice(&mut self, ty: &TypeRef) -> bool {
        let info = self.types_mut().type_ref_info(ty);
        // If there's any out-buffers transitively then a mutable slice is
        // required because the out-buffers could be modified. Otherwise a
        // mutable slice is also required if, transitively, `InBuffer` is used
        // which is used when we're a buffer of a type where not all bits are
        // valid (e.g. the rust representation and the canonical abi may differ).
        info.has_out_buffer || self.has_in_buffer_invalid_bits(ty.type_())
    }

    fn has_in_buffer_invalid_bits(&self, ty: &Type) -> bool {
        match ty {
            Type::Record(r) => r
                .members
                .iter()
                .any(|t| self.has_in_buffer_invalid_bits(t.tref.type_())),
            Type::Variant(v) => v
                .cases
                .iter()
                .filter_map(|c| c.tref.as_ref())
                .any(|t| self.has_in_buffer_invalid_bits(t.type_())),
            Type::List(t) => self.has_in_buffer_invalid_bits(t.type_()),
            Type::Buffer(b) if !b.out && !b.tref.type_().all_bits_valid() => true,
            Type::Buffer(b) => self.has_in_buffer_invalid_bits(b.tref.type_()),
            Type::Builtin(_) | Type::Handle(_) | Type::Pointer(_) | Type::ConstPointer(_) => false,
        }
    }
}

pub fn to_rust_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "type" => "type_",
        "where" => "where_",
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

trait TypeInfoExt {
    fn owns_data(&self) -> bool;
}

impl TypeInfoExt for TypeInfo {
    fn owns_data(&self) -> bool {
        self.has_list || self.has_handle || self.has_in_buffer || self.has_out_buffer
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
