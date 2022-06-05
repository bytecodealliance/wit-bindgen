use heck::*;
use std::fmt;
use wit_bindgen_gen_core::wit_parser::abi::{Bitcast, LiftLower, WasmType};
use wit_bindgen_gen_core::{wit_parser::*, TypeInfo, Types};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeMode {
    Owned,
    AllBorrowed(&'static str),
    LeafBorrowed(&'static str),
    HandlesBorrowed(&'static str),
}

pub trait RustGenerator {
    fn push_str(&mut self, s: &str);
    fn info(&self, ty: TypeId) -> TypeInfo;
    fn types_mut(&mut self) -> &mut Types;
    fn print_borrowed_slice(
        &mut self,
        iface: &Interface,
        mutbl: bool,
        ty: &Type,
        lifetime: &'static str,
    );
    fn print_borrowed_str(&mut self, lifetime: &'static str);
    fn default_param_mode(&self) -> TypeMode;
    fn handle_projection(&self) -> Option<(&'static str, String)>;
    fn handle_wrapper(&self) -> Option<&'static str>;
    fn handle_in_super(&self) -> bool {
        false
    }

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
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        let params = self.print_docs_and_params(iface, func, param_mode, &sig);
        self.push_str(" -> ");
        self.print_ty(iface, &func.result, TypeMode::Owned);
        params
    }

    fn print_docs_and_params(
        &mut self,
        iface: &Interface,
        func: &Function,
        param_mode: TypeMode,
        sig: &FnSig,
    ) -> Vec<String> {
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        // TODO: re-add this when docs are back
        // self.rustdoc_params(&func.results, "Return");

        if !sig.private {
            self.push_str("pub ");
        }
        if sig.unsafe_ {
            self.push_str("unsafe ");
        }
        if sig.async_ {
            self.push_str("async ");
        }
        self.push_str("fn ");
        let func_name = if sig.use_item_name {
            func.item_name()
        } else {
            &func.name
        };
        self.push_str(&to_rust_ident(&func_name));
        if let Some(generics) = &sig.generics {
            self.push_str(generics);
        }
        self.push_str("(");
        if let Some(arg) = &sig.self_arg {
            self.push_str(arg);
            self.push_str(",");
        }
        let mut params = Vec::new();
        for (i, (name, param)) in func.params.iter().enumerate() {
            if i == 0 && sig.self_is_first_param {
                params.push("self".to_string());
                continue;
            }
            let name = to_rust_ident(name);
            self.push_str(&name);
            params.push(name);
            self.push_str(": ");
            self.print_ty(iface, param, param_mode);
            self.push_str(",");
        }
        self.push_str(")");
        params
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

                let suffix = match self.handle_wrapper() {
                    Some(wrapper) => {
                        self.push_str(wrapper);
                        self.push_str("<");
                        ">"
                    }
                    None => "",
                };
                if self.handle_in_super() {
                    self.push_str("super::");
                }
                if let Some((proj, _)) = self.handle_projection() {
                    self.push_str(proj);
                    self.push_str("::");
                }
                self.push_str(&iface.resources[*r].name.to_camel_case());
                self.push_str(suffix);
            }

            Type::Unit => self.push_str("()"),
            Type::Bool => self.push_str("bool"),
            Type::U8 => self.push_str("u8"),
            Type::U16 => self.push_str("u16"),
            Type::U32 => self.push_str("u32"),
            Type::U64 => self.push_str("u64"),
            Type::S8 => self.push_str("i8"),
            Type::S16 => self.push_str("i16"),
            Type::S32 => self.push_str("i32"),
            Type::S64 => self.push_str("i64"),
            Type::Float32 => self.push_str("f32"),
            Type::Float64 => self.push_str("f64"),
            Type::Char => self.push_str("char"),
            Type::String => match mode {
                TypeMode::AllBorrowed(lt) | TypeMode::LeafBorrowed(lt) => {
                    self.print_borrowed_str(lt)
                }
                TypeMode::Owned | TypeMode::HandlesBorrowed(_) => self.push_str("String"),
            },
        }
    }

    fn print_tyid(&mut self, iface: &Interface, id: TypeId, mode: TypeMode) {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        let ty = &iface.types[id];
        match ty {
            CustomType::Named(_) => {
                let name = if lt.is_some() {
                    self.param_name(iface, id)
                } else {
                    self.result_name(iface, id)
                };
                self.push_str(&name);

                // Print generics for the type if it needs them.
                // (This only puts them there if they're needed).
                self.print_generics(&info, lt, false);
            }
            CustomType::Anonymous(ty) => match ty {
                AnonymousType::Option(t) => {
                    self.push_str("Option<");
                    self.print_ty(iface, t, mode);
                    self.push_str(">");
                }

                AnonymousType::Expected(e) => {
                    self.push_str("Result<");
                    self.print_ty(iface, &e.ok, mode);
                    self.push_str(",");
                    self.print_ty(iface, &e.err, mode);
                    self.push_str(">");
                }

                // Tuple-like records are mapped directly to Rust tuples of
                // types. Note the trailing comma after each member to
                // appropriately handle 1-tuples.
                AnonymousType::Tuple(t) => {
                    self.push_str("(");
                    for ty in t.types.iter() {
                        self.print_ty(iface, ty, mode);
                        self.push_str(",");
                    }
                    self.push_str(")");
                }

                AnonymousType::List(t) => self.print_list(iface, t, mode),
            },
        }
    }

    fn print_list(&mut self, iface: &Interface, ty: &Type, mode: TypeMode) {
        match mode {
            TypeMode::AllBorrowed(lt) => {
                self.print_borrowed_slice(iface, false, ty, lt);
            }
            TypeMode::LeafBorrowed(lt) => {
                if iface.all_bits_valid(ty) {
                    self.print_borrowed_slice(iface, false, ty, lt);
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
            if !info.owns_data() {
                self.push_str("#[repr(C)]\n");
                self.push_str("#[derive(Copy, Clone)]\n");
            } else if !info.has_handle {
                self.push_str("#[derive(Clone)]\n");
            }
            self.push_str(&format!("pub struct {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" {\n");
            for field in record.fields.iter() {
                self.rustdoc(&field.docs);
                self.push_str("pub ");
                self.push_str(&to_rust_ident(&field.name));
                self.push_str(": ");
                self.print_ty(iface, &field.ty, mode);
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.push_str("impl");
            self.print_generics(&info, lt, true);
            self.push_str(" std::fmt::Debug for ");
            self.push_str(&name);
            self.print_generics(&info, lt, false);
            self.push_str(" {\n");
            self.push_str("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
            self.push_str(&format!("f.debug_struct(\"{}\")", name));
            for field in record.fields.iter() {
                self.push_str(&format!(
                    ".field(\"{}\", &self.{})",
                    field.name,
                    to_rust_ident(&field.name)
                ));
            }
            self.push_str(".finish()");
            self.push_str("}\n");
            self.push_str("}\n");
        }
    }

    fn print_typedef_tuple(&mut self, iface: &Interface, id: TypeId, tuple: &Tuple, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(iface, id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str(" = (");
            for ty in tuple.types.iter() {
                self.print_ty(iface, ty, mode);
                self.push_str(",");
            }
            self.push_str(");\n");
        }
    }

    fn print_typedef_variant(
        &mut self,
        iface: &Interface,
        id: TypeId,
        variant: &Variant,
        docs: &Docs,
    ) where
        Self: Sized,
    {
        self.print_rust_enum(
            iface,
            id,
            variant
                .cases
                .iter()
                .map(|c| (c.name.to_camel_case(), &c.docs, &c.ty)),
            docs,
        );
    }

    fn print_typedef_union(&mut self, iface: &Interface, id: TypeId, union: &Union, docs: &Docs)
    where
        Self: Sized,
    {
        self.print_rust_enum(
            iface,
            id,
            union
                .cases
                .iter()
                .enumerate()
                .map(|(i, c)| (format!("V{i}"), &c.docs, &c.ty)),
            docs,
        );
    }

    fn print_rust_enum<'a>(
        &mut self,
        iface: &Interface,
        id: TypeId,
        cases: impl IntoIterator<Item = (String, &'a Docs, &'a Type)> + Clone,
        docs: &Docs,
    ) where
        Self: Sized,
    {
        let info = self.info(id);

        for (name, mode) in self.modes_of(iface, id) {
            let name = name.to_camel_case();
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            if !info.owns_data() {
                self.push_str("#[derive(Clone, Copy)]\n");
            } else if !info.has_handle {
                self.push_str("#[derive(Clone)]\n");
            }
            self.push_str(&format!("pub enum {name}"));
            self.print_generics(&info, lt, true);
            self.push_str("{\n");
            for (case_name, docs, payload) in cases.clone() {
                self.rustdoc(docs);
                self.push_str(&case_name);
                if *payload != Type::Unit {
                    self.push_str("(");
                    self.print_ty(iface, payload, mode);
                    self.push_str(")")
                }
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.print_rust_enum_debug(
                id,
                mode,
                &name,
                cases
                    .clone()
                    .into_iter()
                    .map(|(name, _docs, ty)| (name, ty)),
            );
        }
    }

    fn print_rust_enum_debug<'a>(
        &mut self,
        id: TypeId,
        mode: TypeMode,
        name: &str,
        cases: impl IntoIterator<Item = (String, &'a Type)>,
    ) where
        Self: Sized,
    {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        self.push_str("impl");
        self.print_generics(&info, lt, true);
        self.push_str(" std::fmt::Debug for ");
        self.push_str(name);
        self.print_generics(&info, lt, false);
        self.push_str(" {\n");
        self.push_str("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
        self.push_str("match self {\n");
        for (case_name, payload) in cases {
            self.push_str(name);
            self.push_str("::");
            self.push_str(&case_name);
            if *payload != Type::Unit {
                self.push_str("(e)");
            }
            self.push_str(" => {\n");
            self.push_str(&format!("f.debug_tuple(\"{}::{}\")", name, case_name));
            if *payload != Type::Unit {
                self.push_str(".field(e)");
            }
            self.push_str(".finish()\n");
            self.push_str("}\n");
        }
        self.push_str("}\n");
        self.push_str("}\n");
        self.push_str("}\n");
    }

    fn print_typedef_option(&mut self, iface: &Interface, id: TypeId, payload: &Type, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(iface, id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str("= Option<");
            self.print_ty(iface, payload, mode);
            self.push_str(">;\n");
        }
    }

    fn print_typedef_expected(
        &mut self,
        iface: &Interface,
        id: TypeId,
        expected: &Expected,
        docs: &Docs,
    ) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(iface, id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(&info, lt, true);
            self.push_str("= Result<");
            self.print_ty(iface, &expected.ok, mode);
            self.push_str(",");
            self.print_ty(iface, &expected.err, mode);
            self.push_str(">;\n");
        }
    }

    fn print_typedef_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs)
    where
        Self: Sized,
    {
        // TODO: should this perhaps be an attribute in the wit file?
        let is_error = name.contains("errno");

        let name = name.to_camel_case();
        self.rustdoc(docs);
        self.push_str("#[repr(");
        self.int_repr(enum_.tag());
        self.push_str(")]\n#[derive(Clone, Copy, PartialEq, Eq)]\n");
        self.push_str(&format!("pub enum {} {{\n", name.to_camel_case()));
        for case in enum_.cases.iter() {
            self.rustdoc(&case.docs);
            self.push_str(&case.name.to_camel_case());
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
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_camel_case());
                self.push_str(" => \"");
                self.push_str(case.name.as_str());
                self.push_str("\",\n");
            }
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("pub fn message(&self) -> &'static str {\n");
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_camel_case());
                self.push_str(" => \"");
                if let Some(contents) = &case.docs.contents {
                    self.push_str(contents.trim());
                }
                self.push_str("\",\n");
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
            self.push_str("\")\n");
            self.push_str(".field(\"code\", &(*self as i32))\n");
            self.push_str(".field(\"name\", &self.name())\n");
            self.push_str(".field(\"message\", &self.message())\n");
            self.push_str(".finish()\n");
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
        } else {
            self.print_rust_enum_debug(
                id,
                TypeMode::Owned,
                &name,
                enum_
                    .cases
                    .iter()
                    .map(|c| (c.name.to_camel_case(), &Type::Unit)),
            )
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

    fn param_name(&self, iface: &Interface, ty: TypeId) -> String {
        let info = self.info(ty);
        // FIXME: somehow make this statically take a named type
        // while still being able to get at the type info.
        let name = match &iface.types[ty] {
            CustomType::Named(ty) => ty.name.to_camel_case(),
            _ => unreachable!(),
        };
        if self.uses_two_names(&info) {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, iface: &Interface, ty: TypeId) -> String {
        let info = self.info(ty);
        let name = match &iface.types[ty] {
            CustomType::Named(ty) => ty.name.to_camel_case(),
            _ => unreachable!(),
        };
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
                if info.has_list || info.has_handle =>
            {
                Some(s)
            }
            TypeMode::HandlesBorrowed(s) if info.has_handle => Some(s),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct FnSig {
    pub async_: bool,
    pub unsafe_: bool,
    pub private: bool,
    pub use_item_name: bool,
    pub generics: Option<String>,
    pub self_arg: Option<String>,
    pub self_is_first_param: bool,
}

pub trait RustFunctionGenerator {
    fn push_str(&mut self, s: &str);
    fn tmp(&mut self) -> usize;
    fn rust_gen(&self) -> &dyn RustGenerator;
    fn lift_lower(&self) -> LiftLower;

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

    fn record_lower(
        &mut self,
        iface: &Interface,
        id: TypeId,
        record: &Record,
        operand: &str,
        results: &mut Vec<String>,
    ) {
        let tmp = self.tmp();
        self.push_str("let ");
        let name = self.typename_lower(iface, id);
        self.push_str(&name);
        self.push_str("{ ");
        for field in record.fields.iter() {
            let name = to_rust_ident(&field.name);
            let arg = format!("{}{}", name, tmp);
            self.push_str(&name);
            self.push_str(":");
            self.push_str(&arg);
            self.push_str(", ");
            results.push(arg);
        }
        self.push_str("} = ");
        self.push_str(operand);
        self.push_str(";\n");
    }

    fn record_lift(
        &mut self,
        iface: &Interface,
        id: TypeId,
        ty: &Record,
        operands: &[String],
        results: &mut Vec<String>,
    ) {
        let mut result = self.typename_lift(iface, id);
        result.push_str("{");
        for (field, val) in ty.fields.iter().zip(operands) {
            result.push_str(&to_rust_ident(&field.name));
            result.push_str(":");
            result.push_str(&val);
            result.push_str(", ");
        }
        result.push_str("}");
        results.push(result);
    }

    fn tuple_lower(&mut self, tuple: &Tuple, operand: &str, results: &mut Vec<String>) {
        let tmp = self.tmp();
        self.push_str("let (");
        for i in 0..tuple.types.len() {
            let arg = format!("t{}_{}", tmp, i);
            self.push_str(&arg);
            self.push_str(", ");
            results.push(arg);
        }
        self.push_str(") = ");
        self.push_str(operand);
        self.push_str(";\n");
    }

    fn tuple_lift(&mut self, operands: &[String], results: &mut Vec<String>) {
        if operands.len() == 1 {
            results.push(format!("({},)", operands[0]));
        } else {
            results.push(format!("({})", operands.join(", ")));
        }
    }

    fn typename_lower(&self, iface: &Interface, id: TypeId) -> String {
        match self.lift_lower() {
            LiftLower::LowerArgsLiftResults => self.rust_gen().param_name(iface, id),
            LiftLower::LiftArgsLowerResults => self.rust_gen().result_name(iface, id),
        }
    }

    fn typename_lift(&self, iface: &Interface, id: TypeId) -> String {
        match self.lift_lower() {
            LiftLower::LiftArgsLowerResults => self.rust_gen().param_name(iface, id),
            LiftLower::LowerArgsLiftResults => self.rust_gen().result_name(iface, id),
        }
    }
}

pub fn to_rust_ident(name: &str) -> String {
    match name {
        "in" => "in_".into(),
        "type" => "type_".into(),
        "where" => "where_".into(),
        "yield" => "yield_".into(),
        "async" => "async_".into(),
        "self" => "self_".into(),
        s => s.to_snake_case(),
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
        self.has_list || self.has_handle
    }
}

pub fn bitcast(casts: &[Bitcast], operands: &[String], results: &mut Vec<String>) {
    for (cast, operand) in casts.iter().zip(operands) {
        results.push(match cast {
            Bitcast::None => operand.clone(),
            Bitcast::I32ToI64 => format!("i64::from({})", operand),
            Bitcast::F32ToI32 => format!("({}).to_bits() as i32", operand),
            Bitcast::F64ToI64 => format!("({}).to_bits() as i64", operand),
            Bitcast::I64ToI32 => format!("{} as i32", operand),
            Bitcast::I32ToF32 => format!("f32::from_bits({} as u32)", operand),
            Bitcast::I64ToF64 => format!("f64::from_bits({} as u64)", operand),
            Bitcast::F32ToI64 => format!("i64::from(({}).to_bits())", operand),
            Bitcast::I64ToF32 => format!("f32::from_bits({} as u32)", operand),
        });
    }
}

pub enum RustFlagsRepr {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl RustFlagsRepr {
    pub fn new(f: &Flags) -> RustFlagsRepr {
        match f.repr() {
            FlagsRepr::U8 => RustFlagsRepr::U8,
            FlagsRepr::U16 => RustFlagsRepr::U16,
            FlagsRepr::U32(1) => RustFlagsRepr::U32,
            FlagsRepr::U32(2) => RustFlagsRepr::U64,
            FlagsRepr::U32(3 | 4) => RustFlagsRepr::U128,
            FlagsRepr::U32(n) => panic!("unsupported number of flags: {}", n * 32),
        }
    }
}

impl fmt::Display for RustFlagsRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustFlagsRepr::U8 => "u8".fmt(f),
            RustFlagsRepr::U16 => "u16".fmt(f),
            RustFlagsRepr::U32 => "u32".fmt(f),
            RustFlagsRepr::U64 => "u64".fmt(f),
            RustFlagsRepr::U128 => "u128".fmt(f),
        }
    }
}
