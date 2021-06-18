use heck::*;
use witx_bindgen_gen_core::witx2::abi::{Bitcast, Direction, LiftLower, WasmType};
use witx_bindgen_gen_core::{witx2::*, TypeInfo, Types};

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
    Private,
}

pub trait TypePrint {
    fn krate(&self) -> &'static str;
    fn tmp(&mut self) -> usize;
    fn push_str(&mut self, s: &str);
    fn info(&self, ty: TypeId) -> TypeInfo;
    fn types_mut(&mut self) -> &mut Types;
    fn print_usize(&mut self);
    fn print_pointer(&mut self, iface: &Interface, const_: bool, ty: &Type);
    fn print_borrowed_slice(
        &mut self,
        iface: &Interface,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
    );
    fn print_borrowed_str(&mut self, lifetime: &'static str);
    fn direction(&self) -> Direction;
    fn lift_lower(&self) -> LiftLower;
    fn default_param_mode(&self) -> TypeMode;
    fn handle_projection(&self) -> Option<(&'static str, String)>;

    fn rustdoc(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.trim().lines() {
            self.push_str("/// ");
            self.push_str(line);
            self.push_str("\n");
        }
    }

    fn rustdoc_params(&mut self, docs: &[(String, Type)], header: &str) {
        drop((docs, header));
        // let docs = docs
        //     .iter()
        //     .filter(|param| param.docs.trim().len() > 0)
        //     .collect::<Vec<_>>();
        // if docs.len() == 0 {
        //     return;
        // }

        // self.push_str("///\n");
        // self.push_str("/// ## ");
        // self.push_str(header);
        // self.push_str("\n");
        // self.push_str("///\n");

        // for param in docs {
        //     for (i, line) in param.docs.lines().enumerate() {
        //         self.push_str("/// ");
        //         // Currently wasi only has at most one return value, so there's no
        //         // need to indent it or name it.
        //         if header != "Return" {
        //             if i == 0 {
        //                 self.push_str("* `");
        //                 self.push_str(to_rust_ident(param.name.as_str()));
        //                 self.push_str("` - ");
        //             } else {
        //                 self.push_str("  ");
        //             }
        //         }
        //         self.push_str(line);
        //         self.push_str("\n");
        //     }
        // }
    }

    fn print_signature(
        &mut self,
        iface: &Interface,
        func: &Function,
        visibility: Visibility,
        unsafe_: bool,
        self_arg: Option<&str>,
        param_mode: TypeMode,
    ) -> Vec<String> {
        let params =
            self.print_docs_and_params(iface, func, visibility, unsafe_, self_arg, param_mode);
        if func.results.len() > 0 {
            self.push_str(" -> ");
            self.print_results(iface, func);
        }
        params
    }

    fn print_docs_and_params(
        &mut self,
        iface: &Interface,
        func: &Function,
        visibility: Visibility,
        unsafe_: bool,
        self_arg: Option<&str>,
        param_mode: TypeMode,
    ) -> Vec<String> {
        let rust_name = func.name.to_snake_case();
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        // self.rustdoc_params(&func.results, "Return"); // TODO

        match visibility {
            Visibility::Pub => self.push_str("pub "),
            Visibility::Private => (),
        }
        if unsafe_ {
            self.push_str("unsafe ");
        }
        self.push_str("fn ");
        self.push_str(to_rust_ident(&rust_name));

        self.push_str("(");
        if let Some(arg) = self_arg {
            self.push_str(arg);
            self.push_str(",");
        }
        let mut params = Vec::new();
        for (name, param) in func.params.iter() {
            self.push_str(to_rust_ident(name.as_str()));
            params.push(to_rust_ident(name.as_str()).to_string());
            self.push_str(": ");
            self.print_ty(iface, param, param_mode);
            self.push_str(",");
        }
        self.push_str(")");
        params
    }

    fn print_results(&mut self, iface: &Interface, func: &Function) {
        match func.results.len() {
            0 => self.push_str("()"),
            1 => {
                self.print_ty(iface, &func.results[0].1, TypeMode::Owned);
            }
            _ => {
                self.push_str("(");
                for (_, result) in func.results.iter() {
                    self.print_ty(iface, result, TypeMode::Owned);
                    self.push_str(", ");
                }
                self.push_str(")");
            }
        }
    }

    fn print_ty(&mut self, iface: &Interface, ty: &Type, mode: TypeMode) {
        match ty {
            Type::Id(t) => self.print_tyid(iface, *t, mode),
            Type::Handle(r) => {
                let mut info = TypeInfo::default();
                info.has_handle = true;
                let lt = self.lifetime_for(&info, mode);
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
                self.push_str(&iface.resources[*r].name.to_camel_case());
            }

            Type::U8 => self.push_str("u8"),
            Type::CChar => self.push_str("u8"),
            Type::U16 => self.push_str("u16"),
            Type::U32 => self.push_str("u32"),
            Type::Usize => self.print_usize(),
            Type::U64 => self.push_str("u64"),
            Type::S8 => self.push_str("i8"),
            Type::S16 => self.push_str("i16"),
            Type::S32 => self.push_str("i32"),
            Type::S64 => self.push_str("i64"),
            Type::F32 => self.push_str("f32"),
            Type::F64 => self.push_str("f64"),
            Type::Char => self.push_str("char"),
        }
    }

    fn print_tyid(&mut self, iface: &Interface, id: TypeId, mode: TypeMode) {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        let ty = &iface.types[id];
        if ty.name.is_some() {
            let name = if lt.is_some() {
                self.param_name(iface, id)
            } else {
                self.result_name(iface, id)
            };
            self.push_str(&name);

            // If the type recursively owns data and it's a
            // variant/record/list, then we need to place the
            // lifetime parameter on the type as well.
            if info.owns_data() && needs_generics(iface, &ty.kind) {
                self.print_generics(&info, lt, false);
            }

            return;

            fn needs_generics(iface: &Interface, ty: &TypeDefKind) -> bool {
                match ty {
                    TypeDefKind::Variant(_)
                    | TypeDefKind::Record(_)
                    | TypeDefKind::List(_)
                    | TypeDefKind::PushBuffer(_)
                    | TypeDefKind::PullBuffer(_) => true,
                    TypeDefKind::Type(Type::Id(t)) => needs_generics(iface, &iface.types[*t].kind),
                    TypeDefKind::Type(Type::Handle(_)) => true,
                    _ => false,
                }
            }
        }

        match &ty.kind {
            TypeDefKind::List(t) => self.print_list(iface, t, mode),

            TypeDefKind::Pointer(t) => self.print_pointer(iface, false, t),
            TypeDefKind::ConstPointer(t) => self.print_pointer(iface, true, t),

            // Variants can be printed natively if they're `Option`,
            // `Result` , or `bool`, otherwise they must be named for now.
            TypeDefKind::Variant(v) if v.is_bool() => self.push_str("bool"),
            TypeDefKind::Variant(v) => match v.as_expected() {
                Some((ok, err)) => {
                    self.push_str("Result<");
                    match ok {
                        Some(ty) => self.print_ty(iface, ty, mode),
                        None => self.push_str("()"),
                    }
                    self.push_str(",");
                    match err {
                        Some(ty) => self.print_ty(iface, ty, mode),
                        None => self.push_str("()"),
                    }
                    self.push_str(">");
                }
                None => match v.as_option() {
                    Some(ty) => {
                        self.push_str("Option<");
                        self.print_ty(iface, ty, mode);
                        self.push_str(">");
                    }
                    None => panic!("unsupported anonymous variant"),
                },
            },

            // Tuple-like records are mapped directly to Rust tuples of
            // types. Note the trailing comma after each member to
            // appropriately handle 1-tuples.
            TypeDefKind::Record(r) if r.is_tuple() => {
                self.push_str("(");
                for field in r.fields.iter() {
                    self.print_ty(iface, &field.ty, mode);
                    self.push_str(",");
                }
                self.push_str(")");
            }
            TypeDefKind::Record(_) => {
                panic!("unsupported anonymous type reference: record")
            }

            TypeDefKind::PushBuffer(r) => self.print_buffer(iface, true, r, mode),
            TypeDefKind::PullBuffer(r) => self.print_buffer(iface, false, r, mode),

            TypeDefKind::Type(t) => self.print_ty(iface, t, mode),
        }
    }

    fn print_list(&mut self, iface: &Interface, ty: &Type, mode: TypeMode) {
        match ty {
            Type::Char => match mode {
                TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) => {
                    self.print_borrowed_str(lt)
                }
                TypeMode::Owned | TypeMode::HandlesBorrowed(_) => self.push_str("String"),
            },
            t => match mode {
                TypeMode::AllBorrowed(lt) => {
                    let mutbl = self.needs_mutable_slice(iface, ty);
                    self.print_borrowed_slice(iface, mutbl, ty, lt);
                }
                TypeMode::LeafBorrowed(lt) => {
                    if iface.all_bits_valid(t) {
                        let mutbl = self.needs_mutable_slice(iface, ty);
                        self.print_borrowed_slice(iface, mutbl, ty, lt);
                    } else {
                        self.push_str("Vec<");
                        self.print_ty(iface, ty, mode);
                        self.push_str(">");
                    }
                }
                TypeMode::HandlesBorrowed(_) | TypeMode::Owned => {
                    self.push_str("Vec<");
                    self.print_ty(iface, ty, mode);
                    self.push_str(">");
                }
            },
        }
    }

    fn print_buffer(&mut self, iface: &Interface, push: bool, ty: &Type, mode: TypeMode) {
        let lt = match mode {
            TypeMode::AllBorrowed(s) | TypeMode::HandlesBorrowed(s) | TypeMode::LeafBorrowed(s) => {
                s
            }
            TypeMode::Owned => unimplemented!(),
        };
        let prefix = if push { "Push" } else { "Pull" };
        match (self.direction(), self.lift_lower()) {
            // Native exports means rust-compiled-to-wasm exporting something,
            // and buffers there are all using handles, so they use special types.
            (Direction::Export, LiftLower::LiftArgsLowerResults) => {
                let krate = self.krate();
                self.push_str(krate);
                self.push_str("::exports::");
                self.push_str(prefix);
                self.push_str("Buffer");
                if iface.all_bits_valid(ty) {
                    self.push_str("Raw");
                }
                self.push_str("<");
                self.push_str(lt);
                self.push_str(", ");
                self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                self.push_str(">");
            }

            // Wasm exports means host Rust is calling wasm. If all bits are
            // valid we use raw slices (e.g. u8/u64/etc). Otherwise input
            // buffers (input to wasm) is `ExactSizeIterator` and output buffers
            // (output from wasm) is `&mut Vec`
            (Direction::Export, LiftLower::LowerArgsLiftResults) => {
                if iface.all_bits_valid(ty) {
                    self.print_borrowed_slice(iface, push, ty, lt);
                } else if push {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                    }
                    self.push_str(" mut Vec<");
                    self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                    self.push_str(">");
                } else {
                    self.push_str("&");
                    if lt != "'_" {
                        self.push_str(lt);
                    }
                    self.push_str(" mut (dyn ExactSizeIterator<Item = ");
                    self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                    self.push_str(">");
                    if lt != "'_" {
                        self.push_str(" + ");
                        self.push_str(lt);
                    }
                    self.push_str(")");
                }
            }

            (Direction::Import, _) => {
                if iface.all_bits_valid(ty) {
                    self.print_borrowed_slice(iface, push, ty, lt);
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
                    self.print_ty(iface, ty, if push { TypeMode::Owned } else { mode });
                    self.push_str(">");
                }
            }
        }
    }

    fn print_rust_slice(
        &mut self,
        iface: &Interface,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
    ) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        if mutbl {
            self.push_str(" mut ");
        }
        self.push_str("[");
        self.print_ty(iface, ty, TypeMode::AllBorrowed(lifetime));
        self.push_str("]");
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

    fn int_repr(&mut self, repr: Int) {
        self.push_str(int_repr(repr));
    }

    fn wasm_type(&mut self, ty: WasmType) {
        self.push_str(wasm_type(ty));
    }

    fn modes_of(&self, iface: &Interface, ty: TypeId) -> Vec<(String, TypeMode)> {
        let info = self.info(ty);
        let mut result = Vec::new();
        if info.param {
            result.push((self.param_name(iface, ty), self.default_param_mode()));
        }
        if info.result && (!info.param || self.uses_two_names(&info)) {
            result.push((self.result_name(iface, ty), TypeMode::Owned));
        }
        return result;
    }

    fn print_typedef_record(
        &mut self,
        iface: &Interface,
        id: TypeId,
        record: &Record,
        docs: &Docs,
    ) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(iface, id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            if record.is_tuple() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
                self.push_str(" = (");
                for field in record.fields.iter() {
                    self.print_ty(iface, &field.ty, mode);
                    self.push_str(",");
                }
                self.push_str(");\n");
            } else {
                if info.has_pull_buffer || info.has_push_buffer {
                    // skip copy/clone ...
                } else if !info.owns_data() {
                    self.push_str("#[repr(C)]\n");
                    self.push_str("#[derive(Copy, Clone)]\n");
                } else if !info.has_handle {
                    self.push_str("#[derive(Clone)]\n");
                }
                if !info.has_pull_buffer && !info.has_handle {
                    self.push_str("#[derive(Debug)]\n");
                }
                self.push_str(&format!("pub struct {}", name));
                self.print_generics(&info, lt, true);
                self.push_str(" {\n");
                for field in record.fields.iter() {
                    self.rustdoc(&field.docs);
                    self.push_str("pub ");
                    self.push_str(to_rust_ident(&field.name));
                    self.push_str(": ");
                    self.print_ty(iface, &field.ty, mode);
                    self.push_str(",\n");
                }
                self.push_str("}\n");
            }
        }
    }

    fn print_typedef_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        name: &str,
        variant: &Variant,
        docs: &Docs,
    ) {
        // TODO: should this perhaps be an attribute in the witx file?
        let is_error = name.contains("errno") && variant.is_enum();
        let info = self.info(id);

        for (name, mode) in self.modes_of(iface, id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            if variant.is_bool() {
                self.push_str(&format!("pub type {} = bool;\n", name));
                continue;
            } else if let Some(ty) = variant.as_option() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
                self.push_str("= Option<");
                self.print_ty(iface, ty, mode);
                self.push_str(">;\n");
                continue;
            } else if let Some((ok, err)) = variant.as_expected() {
                self.push_str(&format!("pub type {}", name));
                self.print_generics(&info, lt, true);
                self.push_str("= Result<");
                match ok {
                    Some(ty) => self.print_ty(iface, ty, mode),
                    None => self.push_str("()"),
                }
                self.push_str(",");
                match err {
                    Some(ty) => self.print_ty(iface, ty, mode),
                    None => self.push_str("()"),
                }
                self.push_str(">;\n");
                continue;
            }
            if variant.is_enum() {
                self.push_str("#[repr(");
                self.int_repr(variant.tag);
                self.push_str(")]\n#[derive(Clone, Copy, PartialEq, Eq)]\n");
            } else if info.has_pull_buffer || info.has_push_buffer {
                // skip copy/clone
            } else if !info.owns_data() {
                self.push_str("#[derive(Clone, Copy)]\n");
            }
            if !is_error && !info.has_pull_buffer && !info.has_handle {
                self.push_str("#[derive(Debug)]\n");
            }
            self.push_str(&format!("pub enum {}", name.to_camel_case()));
            self.print_generics(&info, lt, true);
            self.push_str("{\n");
            for case in variant.cases.iter() {
                self.rustdoc(&case.docs);
                self.push_str(&case_name(&case.name));
                if let Some(ty) = &case.ty {
                    self.push_str("(");
                    self.print_ty(iface, ty, mode);
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
                    if let Some(contents) = &case.docs.contents {
                        self.push_str(contents.trim());
                    }
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

    fn print_typedef_alias(&mut self, iface: &Interface, id: TypeId, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(iface, id) {
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            let lt = self.lifetime_for(&info, mode);
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            self.print_ty(iface, ty, mode);
            self.push_str(";\n");
        }
    }

    fn print_type_list(&mut self, iface: &Interface, id: TypeId, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(iface, id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            self.print_list(iface, ty, mode);
            self.push_str(";\n");
        }
    }

    fn print_typedef_buffer(
        &mut self,
        iface: &Interface,
        id: TypeId,
        push: bool,
        ty: &Type,
        docs: &Docs,
    ) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(iface, id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" = ");
            self.print_buffer(iface, push, ty, mode);
            self.push_str(";\n");
        }
    }

    fn record_lower(
        &mut self,
        iface: &Interface,
        id: TypeId,
        record: &Record,
        operand: &str,
        results: &mut Vec<String>,
    ) {
        let tmp = self.tmp();
        if record.is_tuple() {
            self.push_str("let (");
            for i in 0..record.fields.len() {
                let arg = format!("t{}_{}", tmp, i);
                self.push_str(&arg);
                self.push_str(", ");
                results.push(arg);
            }
            self.push_str(") = ");
            self.push_str(operand);
            self.push_str(";\n");
        } else {
            self.push_str("let ");
            let name = self.typename_lower(iface, id);
            self.push_str(&name);
            self.push_str("{ ");
            for field in record.fields.iter() {
                let arg = format!("{}{}", field.name.as_str(), tmp);
                self.push_str(to_rust_ident(field.name.as_str()));
                self.push_str(":");
                self.push_str(&arg);
                self.push_str(", ");
                results.push(arg);
            }
            self.push_str("} = ");
            self.push_str(operand);
            self.push_str(";\n");
        }
    }

    fn record_lift(
        &mut self,
        iface: &Interface,
        id: TypeId,
        ty: &Record,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        if ty.is_tuple() {
            if operands.len() == 1 {
                results.push(format!("({},)", operands[0]));
            } else {
                results.push(format!("({})", operands.join(", ")));
            }
        } else {
            let mut result = self.typename_lift(iface, id);
            result.push_str("{");
            for (field, val) in ty.fields.iter().zip(operands) {
                result.push_str(to_rust_ident(&field.name));
                result.push_str(":");
                result.push_str(&val);
                result.push_str(", ");
            }
            result.push_str("}");
            results.push(result);
        }
    }

    fn typename_lower(&self, iface: &Interface, id: TypeId) -> String {
        match self.lift_lower() {
            LiftLower::LowerArgsLiftResults => self.param_name(iface, id),
            LiftLower::LiftArgsLowerResults => self.result_name(iface, id),
        }
    }

    fn typename_lift(&self, iface: &Interface, id: TypeId) -> String {
        match self.lift_lower() {
            LiftLower::LiftArgsLowerResults => self.param_name(iface, id),
            LiftLower::LowerArgsLiftResults => self.result_name(iface, id),
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
        iface: &Interface,
        id: TypeId,
        ty: &Variant,
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
        let has_name = iface.types[id].name.is_some();
        if nresults == 1 && has_name && ty.cases.iter().all(|c| c.ty.is_none()) {
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
                self.push_str(&case.name.to_camel_case());
                self.push_str("(");
                self.push_str(if case.ty.is_some() { "e" } else { "()" });
                self.push_str(")");
            } else if ty.as_option().is_some() {
                self.push_str(&case.name.to_camel_case());
                if case.ty.is_some() {
                    self.push_str("(e)");
                }
            } else if has_name {
                let name = self.typename_lower(iface, id);
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case_name(&case.name));
                if case.ty.is_some() {
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
        iface: &Interface,
        id: TypeId,
        ty: &Variant,
        case: &Case,
        block: &str,
        result: &mut String,
    ) {
        if ty.is_bool() {
            result.push_str(case.name.as_str());
        } else if ty.as_expected().is_some() {
            result.push_str(&case.name.to_camel_case());
            result.push_str("(");
            result.push_str(block);
            result.push_str(")");
        } else if ty.as_option().is_some() {
            result.push_str(&case.name.to_camel_case());
            if case.ty.is_some() {
                result.push_str("(");
                result.push_str(block);
                result.push_str(")");
            }
        } else if iface.types[id].name.is_some() {
            result.push_str(&self.typename_lift(iface, id));
            result.push_str("::");
            result.push_str(&case_name(&case.name));
            if case.ty.is_some() {
                result.push_str("(");
                result.push_str(block);
                result.push_str(")");
            }
        } else {
            unimplemented!()
        }
    }

    fn param_name(&self, iface: &Interface, ty: TypeId) -> String {
        let info = self.info(ty);
        let name = iface.types[ty].name.as_ref().unwrap().to_camel_case();
        if self.uses_two_names(&info) {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, iface: &Interface, ty: TypeId) -> String {
        let info = self.info(ty);
        let name = iface.types[ty].name.as_ref().unwrap().to_camel_case();
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
                    || info.has_push_buffer
                    || info.has_pull_buffer =>
            {
                Some(s)
            }
            TypeMode::HandlesBorrowed(s)
                if info.has_handle || info.has_pull_buffer || info.has_push_buffer =>
            {
                Some(s)
            }
            _ => None,
        }
    }

    fn needs_mutable_slice(&mut self, iface: &Interface, ty: &Type) -> bool {
        let info = self.types_mut().type_info(iface, ty);
        // If there's any out-buffers transitively then a mutable slice is
        // required because the out-buffers could be modified. Otherwise a
        // mutable slice is also required if, transitively, `InBuffer` is used
        // which is used when we're a buffer of a type where not all bits are
        // valid (e.g. the rust representation and the canonical abi may differ).
        info.has_push_buffer || self.has_pull_buffer_invalid_bits(iface, ty)
    }

    fn has_pull_buffer_invalid_bits(&self, iface: &Interface, ty: &Type) -> bool {
        let id = match ty {
            Type::Id(id) => *id,
            _ => return false,
        };
        match &iface.types[id].kind {
            TypeDefKind::Type(t)
            | TypeDefKind::Pointer(t)
            | TypeDefKind::ConstPointer(t)
            | TypeDefKind::PushBuffer(t)
            | TypeDefKind::List(t) => self.has_pull_buffer_invalid_bits(iface, t),
            TypeDefKind::Record(r) => r
                .fields
                .iter()
                .any(|t| self.has_pull_buffer_invalid_bits(iface, &t.ty)),
            TypeDefKind::Variant(v) => v
                .cases
                .iter()
                .filter_map(|c| c.ty.as_ref())
                .any(|t| self.has_pull_buffer_invalid_bits(iface, t)),
            TypeDefKind::PullBuffer(t) => {
                !iface.all_bits_valid(t) || self.has_pull_buffer_invalid_bits(iface, t)
            }
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

pub fn int_repr(repr: Int) -> &'static str {
    match repr {
        Int::U8 => "u8",
        Int::U16 => "u16",
        Int::U32 => "u32",
        Int::U64 => "u64",
    }
}

trait TypeInfoExt {
    fn owns_data(&self) -> bool;
}

impl TypeInfoExt for TypeInfo {
    fn owns_data(&self) -> bool {
        self.has_list || self.has_handle || self.has_pull_buffer || self.has_push_buffer
    }
}

pub fn case_name(id: &str) -> String {
    if id.chars().next().unwrap().is_alphabetic() {
        id.to_camel_case()
    } else {
        format!("V{}", id)
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
