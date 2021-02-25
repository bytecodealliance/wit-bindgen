use heck::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_core::{witx::*, Files, Generator};

#[derive(Default)]
pub struct RustWasm {
    src: String,
    opts: Opts,
    needs_mem: bool,
    type_info: HashMap<Id, TypeInfo>,
}

#[derive(Default)]
struct Opts {
    rustfmt: bool,
    multi_module: bool,
    unchecked: bool,
}

#[derive(Default)]
struct TypeInfo {
    param: bool,
    result: bool,
    owns_data: bool,
}

#[derive(Copy, Clone)]
enum TypeMode {
    Owned,
    Borrowed(&'static str),
    Lifetime(&'static str),
}

impl RustWasm {
    pub fn new() -> RustWasm {
        RustWasm::default()
    }

    pub fn rustfmt(&mut self, rustfmt: bool) -> &mut Self {
        self.opts.rustfmt = rustfmt;
        self
    }

    pub fn multi_module(&mut self, multi_module: bool) -> &mut Self {
        self.opts.multi_module = multi_module;
        self
    }

    pub fn unchecked(&mut self, unchecked: bool) -> &mut Self {
        self.opts.unchecked = unchecked;
        self
    }

    fn rustdoc(&mut self, docs: &str) {
        if docs.trim().is_empty() {
            return;
        }
        for line in docs.lines() {
            self.src.push_str("/// ");
            self.src.push_str(line);
            self.src.push_str("\n");
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

        self.src.push_str("///\n");
        self.src.push_str("/// ## ");
        self.src.push_str(header);
        self.src.push_str("\n");
        self.src.push_str("///\n");

        for param in docs {
            for (i, line) in param.docs.lines().enumerate() {
                self.src.push_str("/// ");
                // Currently wasi only has at most one return value, so there's no
                // need to indent it or name it.
                if header != "Return" {
                    if i == 0 {
                        self.src.push_str("* `");
                        self.src.push_str(to_rust_ident(param.name.as_str()));
                        self.src.push_str("` - ");
                    } else {
                        self.src.push_str("  ");
                    }
                }
                self.src.push_str(line);
                self.src.push_str("\n");
            }
        }
    }

    fn int_repr(&mut self, repr: IntRepr) {
        self.src.push_str(int_repr(repr));
    }

    fn wasm_type(&mut self, ty: WasmType) {
        self.src.push_str(wasm_type(ty));
    }

    fn builtin(&mut self, ty: BuiltinType) {
        match ty {
            // A C `char` in Rust we just interpret always as `u8`. It's
            // technically possible to use `std::os::raw::c_char` but that's
            // overkill for the purposes that we'll be using this type for.
            BuiltinType::U8 { lang_c_char: _ } => self.src.push_str("u8"),
            BuiltinType::U16 => self.src.push_str("u16"),
            BuiltinType::U32 {
                lang_ptr_size: false,
            } => self.src.push_str("u32"),
            BuiltinType::U32 {
                lang_ptr_size: true,
            } => self.src.push_str("usize"),
            BuiltinType::U64 => self.src.push_str("u64"),
            BuiltinType::S8 => self.src.push_str("i8"),
            BuiltinType::S16 => self.src.push_str("i16"),
            BuiltinType::S32 => self.src.push_str("i32"),
            BuiltinType::S64 => self.src.push_str("i64"),
            BuiltinType::F32 => self.src.push_str("f32"),
            BuiltinType::F64 => self.src.push_str("f64"),
            BuiltinType::Char => self.src.push_str("char"),
        }
    }

    fn type_ref(&mut self, ty: &TypeRef, mode: TypeMode) {
        match ty {
            TypeRef::Name(t) => {
                let info = &self.type_info[&t.name];

                // If we're a borrowed piece of data then this is a parameter
                // and we need a leading `&` out in front. Note that the leading
                // `&` is skipped for lists whose typedefs already have a
                // leading `&`.
                if info.owns_data {
                    if let TypeMode::Borrowed(lt) = mode {
                        match &**t.type_() {
                            Type::List(_) => {}
                            _ => {
                                self.src.push_str("&");
                                if lt != "'_" {
                                    self.src.push_str(lt);
                                }
                                self.src.push_str(" ");
                            }
                        }
                    }
                }

                self.src.push_str(&t.name.as_str().to_camel_case());

                // If the type recursively owns data and it's a
                // variant/record/list, then we need to place the lifetime
                // parameter on the type as well.
                if info.owns_data {
                    match &**t.type_() {
                        Type::Variant(_) | Type::Record(_) | Type::List(_) => {
                            self.lifetime_param(mode);
                        }
                        _ => {}
                    }
                }
            }
            TypeRef::Value(v) => match &**v {
                Type::Builtin(t) => self.builtin(*t),
                Type::List(t) => self.type_list(t, mode),
                Type::Pointer(t) => self.type_pointer("mut", t),
                Type::ConstPointer(t) => self.type_pointer("const", t),
                Type::Variant(v) if v.is_bool() => self.src.push_str("bool"),
                Type::Variant(v) => match v.as_expected() {
                    Some((ok, err)) => {
                        self.src.push_str("Result<");
                        match ok {
                            Some(ty) => self.type_ref(ty, mode),
                            None => self.src.push_str("()"),
                        }
                        self.src.push_str(",");
                        match err {
                            Some(ty) => self.type_ref(ty, mode),
                            None => self.src.push_str("()"),
                        }
                        self.src.push_str(">");
                    }
                    None => {
                        panic!("unsupported anonymous variant")
                    }
                },
                Type::Record(r) if r.is_tuple() => {
                    self.src.push_str("(");
                    for member in r.members.iter() {
                        self.type_ref(&member.tref, mode);
                        self.src.push_str(",");
                    }
                    self.src.push_str(")");
                }
                t => panic!("unsupported anonymous type reference: {}", t.kind()),
            },
        }
    }

    fn type_list(&mut self, ty: &TypeRef, mode: TypeMode) {
        match &**ty.type_() {
            Type::Builtin(BuiltinType::Char) => match mode {
                TypeMode::Borrowed(lt) | TypeMode::Lifetime(lt) => {
                    self.src.push_str("&");
                    if lt != "'_" {
                        self.src.push_str(lt);
                        self.src.push_str(" ");
                    }
                    self.src.push_str(" str");
                }
                TypeMode::Owned => {
                    self.src.push_str("String");
                }
            },
            _ => match mode {
                TypeMode::Borrowed(lt) | TypeMode::Lifetime(lt) => {
                    self.src.push_str("&");
                    if lt != "'_" {
                        self.src.push_str(lt);
                        self.src.push_str(" ");
                    }
                    self.src.push_str("[");
                    self.type_ref(ty, TypeMode::Lifetime(lt));
                    self.src.push_str("]");
                }
                TypeMode::Owned => {
                    self.src.push_str("Vec<");
                    self.type_ref(ty, TypeMode::Owned);
                    self.src.push_str(">");
                }
            },
        }
    }

    fn type_pointer(&mut self, kind: &str, ty: &TypeRef) {
        self.src.push_str("*");
        self.src.push_str(kind);
        self.src.push_str(" ");
        match &**ty.type_() {
            Type::Builtin(_) | Type::Pointer(_) | Type::ConstPointer(_) => {
                self.type_ref(ty, TypeMode::Owned);
            }
            Type::List(_) | Type::Variant(_) => panic!("unsupported type"),
            Type::Handle(_) | Type::Record(_) => {
                self.needs_mem = true;
                self.src.push_str("mem::ManuallyDrop<");
                self.type_ref(ty, TypeMode::Owned);
                self.src.push_str(">");
            }
        }
    }

    fn lifetime_param(&mut self, mode: TypeMode) {
        match mode {
            TypeMode::Borrowed(lt) | TypeMode::Lifetime(lt) => {
                self.src.push_str("<");
                self.src.push_str(lt);
                self.src.push_str(">");
            }
            TypeMode::Owned => {}
        }
    }

    fn modes_of(&self, ty: &Id) -> Vec<(String, TypeMode)> {
        let info = &self.type_info[ty];
        let mut result = Vec::new();
        if info.owns_data {
            if info.param {
                result.push((info.param_name(ty), TypeMode::Lifetime("'a")));
            }
            if info.result {
                result.push((info.result_name(ty), TypeMode::Lifetime("'a")));
            }
        } else {
            result.push((info.result_name(ty), TypeMode::Owned));
        }
        return result;
    }

    fn register_type_info(&mut self, ty: &TypeRef, param: bool, result: bool) {
        if let TypeRef::Name(nt) = ty {
            let info = self.type_info.get_mut(&nt.name).unwrap();
            info.param = info.param || param;
            info.result = info.result || result;
        }
        let mut owns = false;
        match &**ty.type_() {
            Type::Builtin(_) => {}
            Type::Handle(_) => owns = true,
            Type::List(t) => {
                self.register_type_info(t, param, result);
                owns = true;
            }
            Type::Pointer(t) | Type::ConstPointer(t) => self.register_type_info(t, param, result),
            Type::Variant(v) => {
                for c in v.cases.iter() {
                    if let Some(ty) = &c.tref {
                        self.register_type_info(ty, param, result);
                    }
                }
            }
            Type::Record(r) => {
                for member in r.members.iter() {
                    self.register_type_info(&member.tref, param, result);
                }
            }
        }
        if let TypeRef::Name(nt) = ty {
            let info = self.type_info.get_mut(&nt.name).unwrap();
            info.owns_data = info.owns_data || owns;
        }
    }
}

impl Generator for RustWasm {
    fn preprocess(&mut self, doc: &Document) {
        for t in doc.typenames() {
            self.type_info.insert(t.name.clone(), TypeInfo::default());
        }
        for m in doc.modules() {
            for f in m.funcs() {
                for param in f.params.iter() {
                    self.register_type_info(&param.tref, true, false);
                }
                for param in f.results.iter() {
                    self.register_type_info(&param.tref, false, true);
                }
            }
        }
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
        if let Some(repr) = record.bitflags_repr() {
            let name = name.as_str();
            self.rustdoc(docs);
            self.src
                .push_str(&format!("pub type {} = ", name.to_camel_case()));
            self.int_repr(repr);
            self.src.push(';');
            for (i, member) in record.members.iter().enumerate() {
                self.rustdoc(&member.docs);
                self.src.push_str(&format!(
                    "pub const {}_{}: {} = 1 << {};\n",
                    name.to_shouty_snake_case(),
                    member.name.as_str().to_shouty_snake_case(),
                    name.to_camel_case(),
                    i,
                ));
            }
            return;
        }
        for (name, mode) in self.modes_of(name) {
            if record.members.iter().all(|m| is_clone(&m.tref)) {
                if record.members.iter().all(|m| is_copy(&m.tref)) {
                    self.src.push_str("#[repr(C)]\n");
                    self.src.push_str("#[derive(Copy)]\n");
                }
                self.src.push_str("#[derive(Clone)]\n");
            }
            self.src.push_str("#[derive(Debug)]\n");
            self.src.push_str(&format!("pub struct {} {{\n", name));
            for member in record.members.iter() {
                self.rustdoc(&member.docs);
                self.src.push_str("pub ");
                self.src.push_str(to_rust_ident(member.name.as_str()));
                self.src.push_str(": ");
                self.type_ref(&member.tref, mode);
                self.src.push_str(",\n");
            }
            self.src.push_str("}\n");
        }
    }

    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str) {
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            if variant
                .cases
                .iter()
                .filter_map(|c| c.tref.as_ref())
                .all(is_clone)
            {
                if variant.cases.iter().all(|c| c.tref.is_none()) {
                    self.src.push_str("#[repr(");
                    self.int_repr(variant.tag_repr);
                    self.src.push_str(")]\n#[derive(Copy, PartialEq, Eq)]\n");
                } else if variant
                    .cases
                    .iter()
                    .filter_map(|c| c.tref.as_ref())
                    .all(is_copy)
                {
                    self.src.push_str("#[derive(Copy)]\n");
                }
                self.src.push_str("#[derive(Clone)]\n");
            }
            self.src.push_str("#[derive(Debug)]\n");
            self.src
                .push_str(&format!("pub enum {} {{\n", name.to_camel_case()));
            for case in variant.cases.iter() {
                self.rustdoc(&case.docs);
                self.src.push_str(&case_name(&case.name));
                if let Some(ty) = &case.tref {
                    self.src.push_str("(");
                    self.type_ref(ty, mode);
                    self.src.push_str(")")
                }
                self.src.push_str(",\n");
            }
            self.src.push_str("}\n");
        }
    }

    fn type_handle(&mut self, name: &Id, _ty: &HandleDatatype, docs: &str) {
        self.rustdoc(docs);
        self.src.push_str("#[derive(Debug)]\n");
        self.src.push_str("#[repr(transparent)]\n");
        self.src.push_str(&format!(
            "pub struct {}(i32);",
            name.as_str().to_camel_case()
        ));
        self.src.push_str("impl ");
        self.src.push_str(&name.as_str().to_camel_case());
        self.needs_mem = true;
        self.src.push_str(
            " {
                pub unsafe fn from_raw(raw: i32) -> Self {
                    Self(raw)
                }

                pub fn into_raw(self) -> i32 {
                    let ret = self.0;
                    mem::forget(self);
                    return ret;
                }
            }",
        );
    }

    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.as_str().to_camel_case()));
        self.src.push_str(" = ");
        self.src.push_str(&ty.name.as_str().to_camel_case());
        self.src.push(';');
    }

    fn type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str) {
        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            self.src.push_str(&format!("pub type {}", name));
            self.lifetime_param(mode);
            self.src.push_str(" = ");
            self.type_list(ty, mode);
            self.src.push(';');
        }
    }

    fn type_pointer(&mut self, name: &Id, const_: bool, ty: &TypeRef, docs: &str) {
        self.rustdoc(docs);
        let mutbl = if const_ { "const" } else { "mut" };
        self.src.push_str(&format!(
            "pub type {} = *{} ",
            name.as_str().to_camel_case(),
            mutbl,
        ));
        self.type_ref(ty, TypeMode::Owned);
        self.src.push(';');
    }

    fn type_builtin(&mut self, name: &Id, ty: BuiltinType, docs: &str) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.as_str().to_camel_case()));
        self.src.push_str(" = ");
        self.builtin(ty);
        self.src.push(';');
    }

    fn const_(&mut self, name: &Id, ty: &Id, val: u64, docs: &str) {
        self.rustdoc(docs);
        self.src.push_str(&format!(
            "pub const {}_{}: {} = {};\n",
            ty.as_str().to_shouty_snake_case(),
            name.as_str().to_shouty_snake_case(),
            ty.as_str().to_camel_case(),
            val
        ));
    }

    fn import(&mut self, module: &Id, func: &InterfaceFunc) {
        let rust_name = func.name.as_ref().to_snake_case();
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        self.rustdoc_params(&func.results, "Return");

        self.src.push_str("pub fn ");

        if self.opts.multi_module {
            self.src.push_str(&module.as_str().to_snake_case());
            self.src.push('_');
            self.src.push_str(&rust_name);
        } else {
            self.src.push_str(to_rust_ident(&rust_name));
        }

        self.src.push_str("(");
        let mut params = Vec::new();
        for param in func.params.iter() {
            self.src.push_str(to_rust_ident(param.name.as_str()));
            params.push(to_rust_ident(param.name.as_str()).to_string());
            self.src.push_str(": ");
            self.type_ref(&param.tref, TypeMode::Borrowed("'_"));
            self.src.push_str(",");
        }
        self.src.push_str(")");

        match func.results.len() {
            0 => {}
            1 => {
                self.src.push_str(" -> ");
                self.type_ref(&func.results[0].tref, TypeMode::Owned);
            }
            _ => {
                self.src.push_str(" -> (");
                for result in func.results.iter() {
                    self.type_ref(&result.tref, TypeMode::Owned);
                    self.src.push_str(", ");
                }
                self.src.push_str(")");
            }
        }
        self.src.push_str("{unsafe{");

        func.call(
            module,
            CallMode::DeclaredImport,
            &mut RustWasmBindgen {
                cfg: self,
                params,
                block_storage: Vec::new(),
                blocks: Vec::new(),
                tmp: 0,
            },
        );

        self.src.push_str("}}");
    }

    fn export(&mut self, module: &Id, func: &InterfaceFunc) {
        let rust_name = func.name.as_ref().to_snake_case();

        self.src.push_str("#[export_name = \"");
        self.src.push_str(&rust_name);
        self.src.push_str("\"]\n");
        self.src
            .push_str("pub unsafe extern \"C\" fn __witx_bindgen_");
        self.src.push_str(&rust_name);
        self.src.push_str("(");
        let sig = func.wasm_signature();
        let mut params = Vec::new();
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            self.src.push_str(&name);
            self.src.push_str(": ");
            self.wasm_type(*param);
            self.src.push_str(",");
            params.push(name);
        }
        self.src.push_str(")");

        match sig.results.len() {
            0 => {}
            1 => {
                self.src.push_str(" -> ");
                self.wasm_type(sig.results[0]);
            }
            _ => unimplemented!(),
        }
        self.src.push_str("{");

        func.call(
            module,
            CallMode::DefinedExport,
            &mut RustWasmBindgen {
                cfg: self,
                params,
                block_storage: Vec::new(),
                blocks: Vec::new(),
                tmp: 0,
            },
        );

        self.src.push_str("}");
    }

    fn finish(&mut self) -> Files {
        let mut files = Files::default();

        let mut src = mem::take(&mut self.src);

        if self.needs_mem {
            src.insert_str(0, "use std::mem;\n");
        }

        if self.opts.rustfmt {
            let mut child = Command::new("rustfmt")
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
            src.truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(&mut src)
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }
        files.push("bindings.rs", &src);
        files
    }
}

//fn render_enum_like_variant(src: &mut String, name: &str, s: &Variant) {
//    src.push_str("#[repr(transparent)]\n");
//    src.push_str("#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]\n");
//    src.push_str(&format!("pub struct {}(", name.to_camel_case()));
//    s.tag_repr.render(src);
//    src.push_str(");\n");
//    for (i, variant) in s.cases.iter().enumerate() {
//        rustdoc(&variant.docs, src);
//        src.push_str(&format!(
//            "pub const {}_{}: {ty} = {ty}({});\n",
//            name.to_shouty_snake_case(),
//            variant.name.as_str().to_shouty_snake_case(),
//            i,
//            ty = name.to_camel_case(),
//        ));
//    }
//    let camel_name = name.to_camel_case();

//    src.push_str("impl ");
//    src.push_str(&camel_name);
//    src.push_str("{\n");

//    src.push_str("pub const fn raw(&self) -> ");
//    s.tag_repr.render(src);
//    src.push_str("{ self.0 }\n\n");

//    src.push_str("pub fn name(&self) -> &'static str {\n");
//    src.push_str("match self.0 {");
//    for (i, variant) in s.cases.iter().enumerate() {
//        src.push_str(&i.to_string());
//        src.push_str(" => \"");
//        src.push_str(&variant.name.as_str().to_shouty_snake_case());
//        src.push_str("\",");
//    }
//    src.push_str("_ => unsafe { core::hint::unreachable_unchecked() },");
//    src.push_str("}\n");
//    src.push_str("}\n");

//    src.push_str("pub fn message(&self) -> &'static str {\n");
//    src.push_str("match self.0 {");
//    for (i, variant) in s.cases.iter().enumerate() {
//        src.push_str(&i.to_string());
//        src.push_str(" => \"");
//        src.push_str(variant.docs.trim());
//        src.push_str("\",");
//    }
//    src.push_str("_ => unsafe { core::hint::unreachable_unchecked() },");
//    src.push_str("}\n");
//    src.push_str("}\n");

//    src.push_str("}\n");

//    src.push_str("impl fmt::Debug for ");
//    src.push_str(&camel_name);
//    src.push_str("{\nfn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n");
//    src.push_str("f.debug_struct(\"");
//    src.push_str(&camel_name);
//    src.push_str("\")");
//    src.push_str(".field(\"code\", &self.0)");
//    src.push_str(".field(\"name\", &self.name())");
//    src.push_str(".field(\"message\", &self.message())");
//    src.push_str(".finish()");
//    src.push_str("}\n");
//    src.push_str("}\n");

//    // Auto-synthesize an implementation of the standard `Error` trait for
//    // error-looking types based on their name.
//    //
//    // TODO: should this perhaps be an attribute in the witx file?
//    if name.contains("errno") {
//        src.push_str("impl fmt::Display for ");
//        src.push_str(&camel_name);
//        src.push_str("{\nfn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n");
//        src.push_str("write!(f, \"{} (error {})\", self.name(), self.0)");
//        src.push_str("}\n");
//        src.push_str("}\n");
//        src.push_str("\n");
//        src.push_str("#[cfg(feature = \"std\")]\n");
//        src.push_str("extern crate std;\n");
//        src.push_str("#[cfg(feature = \"std\")]\n");
//        src.push_str("impl std::error::Error for ");
//        src.push_str(&camel_name);
//        src.push_str("{}\n");
//    }
//}

struct RustWasmBindgen<'a> {
    cfg: &'a mut RustWasm,
    params: Vec<String>,
    block_storage: Vec<String>,
    blocks: Vec<String>,
    tmp: usize,
}

impl RustWasmBindgen<'_> {
    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn push_str(&mut self, s: &str) {
        self.cfg.src.push_str(s);
    }
}

impl Bindgen for RustWasmBindgen<'_> {
    type Operand = String;

    fn push_block(&mut self) {
        let prev = mem::take(&mut self.cfg.src);
        self.block_storage.push(prev);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let to_restore = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.cfg.src, to_restore);
        let expr = match operands.len() {
            0 => "()".to_string(),
            1 => operands.pop().unwrap(),
            _ => format!("({})", operands.join(", ")),
        };
        if src.is_empty() {
            self.blocks.push(expr);
        } else {
            self.blocks.push(format!("{{ {}; {} }}", src, expr));
        }
    }

    fn allocate_typed_space(&mut self, ty: &NamedType) -> String {
        let tmp = self.tmp();
        self.cfg.needs_mem = true;
        self.push_str(&format!("let mut rp{} = mem::MaybeUninit::<", tmp));
        self.push_str(&ty.name.as_str().to_camel_case());
        self.push_str(">::uninit();");
        self.push_str(&format!("let ptr{} = rp{0}.as_mut_ptr() as i32;\n", tmp));
        format!("ptr{}", tmp)
    }

    fn allocate_i64_array(&mut self, amt: usize) -> String {
        let tmp = self.tmp();
        self.push_str(&format!("let mut space{} = [0i64; {}];\n", tmp, amt));
        self.push_str(&format!("let ptr{} = space{0}.as_mut_ptr() as i32;\n", tmp));
        format!("ptr{}", tmp)
    }

    fn emit(
        &mut self,
        inst: &Instruction<'_>,
        operands: &mut Vec<String>,
        results: &mut Vec<String>,
    ) {
        let unchecked = self.cfg.opts.unchecked;
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
        };

        let mut let_results = |amt: usize, results: &mut Vec<String>| match amt {
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
        };

        match inst {
            Instruction::GetArg { nth } => results.push(self.params[*nth].clone()),
            Instruction::I32Const { val } => results.push(format!("{}i32", val)),
            Instruction::ConstZero { tys } => {
                for ty in tys.iter() {
                    match ty {
                        WasmType::I32 => results.push("0i32".to_string()),
                        WasmType::I64 => results.push("0i64".to_string()),
                        WasmType::F32 => results.push("0.0f32".to_string()),
                        WasmType::F64 => results.push("0.0f64".to_string()),
                    }
                }
            }

            Instruction::I64FromU64 => top_as("i64"),
            Instruction::I32FromUsize
            | Instruction::I32FromChar
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromChar8
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU32 => top_as("i32"),

            Instruction::F32FromIf32
            | Instruction::F64FromIf64
            | Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::I64FromS64
            | Instruction::I32FromS32
            | Instruction::S32FromI32
            | Instruction::S64FromI64 => {
                results.push(operands.pop().unwrap());
            }
            Instruction::S8FromI32 => top_as("i8"),
            Instruction::Char8FromI32 | Instruction::U8FromI32 => top_as("u8"),
            Instruction::S16FromI32 => top_as("i16"),
            Instruction::U16FromI32 => top_as("u16"),
            Instruction::U32FromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),
            Instruction::UsizeFromI32 => top_as("usize"),
            Instruction::CharFromI32 => {
                if unchecked {
                    results.push(format!(
                        "std::char::from_u32_unchecked({} as u32)",
                        operands[0]
                    ));
                } else {
                    results.push(format!(
                        "std::char::from_u32({} as u32).unwrap()",
                        operands[0]
                    ));
                }
            }

            Instruction::Bitcasts { casts } => {
                for (cast, item) in casts.iter().zip(operands.drain(..)) {
                    match cast {
                        Bitcast::None => results.push(item),
                        Bitcast::F32ToF64 => results.push(format!("f64::from({})", item)),
                        Bitcast::I32ToI64 => results.push(format!("i64::from({})", item)),
                        Bitcast::F32ToI32 => {
                            results.push(format!("({}).to_bits() as i32", item));
                        }
                        Bitcast::F64ToI64 => {
                            results.push(format!("({}).to_bits() as i64", item));
                        }
                        Bitcast::F64ToF32 => results.push(format!("{} as f32", item)),
                        Bitcast::I64ToI32 => results.push(format!("{} as i32", item)),
                        Bitcast::I32ToF32 => {
                            results.push(format!("f32::from_bits({} as u32)", item))
                        }
                        Bitcast::I64ToF64 => {
                            results.push(format!("f64::from_bits({} as u64)", item))
                        }
                        Bitcast::F32ToI64 => {
                            results.push(format!("i64::from(({}).to_bits())", item))
                        }
                        Bitcast::I64ToF32 => {
                            results.push(format!("f32::from_bits({} as u32)", item))
                        }
                    }
                }
            }

            Instruction::I32FromOwnedHandle { .. } => unimplemented!(),
            Instruction::I32FromBorrowedHandle { .. } => {
                results.push(format!("{}.0", operands[0]));
            }
            Instruction::HandleBorrowedFromI32 { .. } => unimplemented!(),
            Instruction::HandleOwnedFromI32 { ty } => {
                results.push(format!(
                    "{}({})",
                    ty.name.as_str().to_camel_case(),
                    operands[0]
                ));
            }

            Instruction::I32FromBitflags { .. } => top_as("i32"),
            Instruction::I64FromBitflags { .. } => top_as("i64"),
            Instruction::BitflagsFromI32 { repr, .. } => top_as(int_repr(*repr)),
            Instruction::BitflagsFromI64 { repr, .. } => top_as(int_repr(*repr)),

            Instruction::RecordLower { ty, name } => {
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
                    self.push_str(&operands[0]);
                    self.push_str(";\n");
                } else if let Some(name) = name {
                    self.push_str("let ");
                    self.push_str(&name.name.as_str().to_camel_case());
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
                    self.push_str(&operands[0]);
                    self.push_str(";\n");
                } else {
                    unimplemented!()
                }
            }
            Instruction::RecordLift { ty, name } => {
                if ty.is_tuple() {
                    if operands.len() == 1 {
                        results.push(format!("({},)", operands[0]));
                    } else {
                        results.push(format!("({})", operands.join(",")));
                    }
                } else if let Some(name) = name {
                    let mut result = name.name.as_str().to_camel_case();
                    result.push_str("{");
                    for (member, val) in ty.members.iter().zip(operands.drain(..)) {
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

            Instruction::VariantPayload => results.push("e".to_string()),

            // If this is a named enum with no type payloads and we're
            // producing a singular result, then we know we're directly
            // converting from the Rust enum to the integer discriminant. In
            // this scenario we can optimize a bit and use just `as i32`
            // instead of letting LLVM figure out it can do the same with
            // optimizing the `match` generated below.
            Instruction::VariantLower {
                ty,
                name: Some(_),
                nresults: 1,
            } if ty.cases.iter().all(|c| c.tref.is_none()) => {
                self.blocks.drain(self.blocks.len() - ty.cases.len()..);
                results.push(format!("{} as i32", operands[0]));
            }

            Instruction::VariantLower { ty, name, nresults } => {
                let_results(*nresults, results);
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                self.push_str("match ");
                self.push_str(&operands[0]);
                self.push_str("{\n");
                for (case, block) in ty.cases.iter().zip(blocks) {
                    if ty.is_bool() {
                        self.push_str(case.name.as_str());
                    } else if ty.as_expected().is_some() {
                        self.push_str(&case.name.as_str().to_camel_case());
                        self.push_str("(");
                        self.push_str(if case.tref.is_some() { "e" } else { "()" });
                        self.push_str(")");
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

            // In unchecked mode when this type is a named enum then we know we
            // defined the type so we can transmute directly into it.
            Instruction::VariantLift {
                ty,
                name: Some(name),
            } if ty.cases.iter().all(|c| c.tref.is_none()) && unchecked => {
                self.blocks.drain(self.blocks.len() - ty.cases.len()..);
                self.cfg.needs_mem = true;
                let mut result = format!("mem::transmute::<_, ");
                result.push_str(&name.name.as_str().to_camel_case());
                result.push_str(">(");
                result.push_str(&operands[0]);
                result.push_str(" as ");
                result.push_str(int_repr(ty.tag_repr));
                result.push_str(")");
                results.push(result);
            }

            Instruction::VariantLift { ty, name } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                let mut result = format!("match ");
                result.push_str(&operands[0]);
                result.push_str(" {\n");
                for (i, (case, block)) in ty.cases.iter().zip(blocks).enumerate() {
                    if i == ty.cases.len() - 1 && unchecked {
                        result.push_str("_");
                    } else {
                        result.push_str(&i.to_string());
                    }
                    result.push_str(" => ");
                    if ty.is_bool() {
                        result.push_str(case.name.as_str());
                    } else if ty.as_expected().is_some() {
                        result.push_str(&case.name.as_str().to_camel_case());
                        result.push_str("(");
                        result.push_str(&block);
                        result.push_str(")");
                    } else if let Some(name) = name {
                        result.push_str(&name.name.as_str().to_camel_case());
                        result.push_str("::");
                        result.push_str(&case_name(&case.name));
                        if case.tref.is_some() {
                            result.push_str("(");
                            result.push_str(&block);
                            result.push_str(")");
                        }
                    } else {
                        unimplemented!()
                    }
                    result.push_str(",\n");
                }
                if !unchecked {
                    result.push_str("_ => panic!(\"invalid enum discriminant\"),");
                }
                result.push_str("}");
                results.push(result);
            }

            Instruction::ListCanonLower { element, malloc } => {
                let tmp = self.tmp();
                let val = format!("vec{}", tmp);
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                if malloc.is_none() {
                    self.push_str(&format!("let {} = {};\n", val, operands[0]));
                } else {
                    let op0 = match &**element.type_() {
                        Type::Builtin(BuiltinType::Char) => {
                            format!("{}.into_bytes()", operands[0])
                        }
                        _ => operands.pop().unwrap(),
                    };
                    self.push_str(&format!("let {} = ({}).into_boxed_slice();\n", val, op0));
                }
                self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, val));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, val));
                if malloc.is_some() {
                    self.cfg.needs_mem = true;
                    self.push_str(&format!("mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListCanonLift { element, .. } => {
                let tmp = self.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                match &**element.type_() {
                    Type::Builtin(BuiltinType::Char) => {
                        if unchecked {
                            results.push(format!("String::from_utf8_unchecked({})", result));
                        } else {
                            results.push(format!("String::from_utf8({}).unwrap()", result));
                        }
                    }
                    _ => results.push(result),
                }
            }

            Instruction::ListLower { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                let size_align = element.mem_size_align();
                let (ty, multiplier) = match size_align {
                    SizeAlign { align: 1, size } => ("u8", size),
                    SizeAlign { align: 2, size } => ("u16", size / 2),
                    SizeAlign { align: 4, size } => ("u32", size / 4),
                    SizeAlign { align: 8, size } => ("u64", size / 8),
                    _ => unimplemented!(),
                };
                self.push_str(&format!(
                    "let {} = Vec::<{}>::with_capacity({}.len() * {});\n",
                    result, ty, vec, multiplier,
                ));
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!(
                    "let base = {}.as_ptr() as i32 + (i as i32) * {};\n",
                    result, size_align.size,
                ));
                self.push_str(&body);
                self.push_str("}");
                let ptr = format!("ptr{}", tmp);
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {}.as_ptr() as i32;\n", ptr, result));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, result));
                self.push_str(&format!("mem::forget({});\n", result));
                self.cfg.needs_mem = true;
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListLift { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size_align = element.mem_size_align();
                let (ty, multiplier) = match size_align {
                    SizeAlign { align: 1, size } => ("u8", size),
                    SizeAlign { align: 2, size } => ("u16", size / 2),
                    SizeAlign { align: 4, size } => ("u32", size / 4),
                    SizeAlign { align: 8, size } => ("u64", size / 8),
                    _ => unimplemented!(),
                };
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1],));
                let base = format!("base{}", tmp);
                self.push_str(&format!(
                    "let {} = Vec::<{}>::from_raw_parts({} as *mut _, {len} * {mult}, {len} * {mult});\n",
                    base, ty, operands[0], len=len, mult=multiplier,
                ));
                let result = format!("result{}", tmp);
                self.push_str(&format!(
                    "let mut {} = Vec::with_capacity({});\n",
                    result, len,
                ));

                self.push_str("for i in 0..");
                self.push_str(&len);
                self.push_str(" {\n");
                self.push_str("let base = ");
                self.push_str(&base);
                self.push_str(".as_ptr() as i32 + (i as i32) *");
                self.push_str(&size_align.size.to_string());
                self.push_str(";\n");
                self.push_str(&result);
                self.push_str(".push(");
                self.push_str(&body);
                self.push_str(");\n");
                self.push_str("}\n");
                results.push(result);
            }

            Instruction::IterElem => results.push("e".to_string()),

            Instruction::IterBasePointer => results.push("base".to_string()),

            Instruction::CallWasm {
                module,
                name,
                params,
                results: func_results,
            } => {
                assert!(func_results.len() < 2);

                // Define the actual function we're calling inline
                self.push_str("#[link(wasm_import_module = \"");
                self.push_str(module);
                self.push_str("\")]\n");
                self.push_str("extern \"C\" {\n");
                self.push_str("#[link_name = \"");
                self.push_str(name);
                self.push_str("\"]\n");
                self.push_str("fn witx_import(");
                for param in params.iter() {
                    self.push_str("_: ");
                    self.push_str(wasm_type(*param));
                    self.push_str(",");
                }
                self.push_str(")");
                for result in func_results.iter() {
                    self.push_str("->");
                    self.push_str(wasm_type(*result));
                }
                self.push_str(";\n}\n");

                // ... then call the function with all our operands
                if func_results.len() > 0 {
                    self.push_str("let ret = ");
                    results.push("ret".to_string());
                }
                self.push_str("witx_import");
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
            }

            Instruction::CallInterface { module: _, func } => {
                let_results(func.results.len(), results);
                self.push_str(func.name.as_str());
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
            }

            Instruction::Return { amt: 0 } => {}
            Instruction::Return { amt: 1 } => {
                self.push_str(&operands[0]);
            }
            Instruction::Return { .. } => {
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(")");
            }

            Instruction::I32Load { offset } => {
                results.push(format!("*(({} + {}) as *const i32)", operands[0], offset));
            }
            Instruction::I32Load8U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load8S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i8))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16U { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const u16))",
                    operands[0], offset
                ));
            }
            Instruction::I32Load16S { offset } => {
                results.push(format!(
                    "i32::from(*(({} + {}) as *const i16))",
                    operands[0], offset
                ));
            }
            Instruction::I64Load { offset } => {
                results.push(format!("*(({} + {}) as *const i64)", operands[0], offset));
            }
            Instruction::F32Load { offset } => {
                results.push(format!("*(({} + {}) as *const f32)", operands[0], offset));
            }
            Instruction::F64Load { offset } => {
                results.push(format!("*(({} + {}) as *const f64)", operands[0], offset));
            }
            Instruction::I32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u8) = ({}) as u8;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut u16) = ({}) as u16;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut i64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F32Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f32) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::F64Store { offset } => {
                self.push_str(&format!(
                    "*(({} + {}) as *mut f64) = {};\n",
                    operands[1], offset, operands[0]
                ));
            }

            Instruction::Witx { instr } => match instr {
                WitxInstruction::I32FromPointer => top_as("i32"),
                WitxInstruction::I32FromConstPointer => top_as("i32"),
                WitxInstruction::ReuseReturn => results.push("ret".to_string()),
                i => unimplemented!("{:?}", i),
            },
        }
    }
}

fn to_rust_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "type" => "type_",
        "yield" => "yield_",
        s => s,
    }
}

fn wasm_type(ty: WasmType) -> &'static str {
    match ty {
        WasmType::I32 => "i32",
        WasmType::I64 => "i64",
        WasmType::F32 => "f32",
        WasmType::F64 => "f64",
    }
}

fn int_repr(repr: IntRepr) -> &'static str {
    match repr {
        IntRepr::U8 => "u8",
        IntRepr::U16 => "u16",
        IntRepr::U32 => "u32",
        IntRepr::U64 => "u64",
    }
}

fn case_name(id: &Id) -> String {
    let s = id.as_str();
    if s.chars().next().unwrap().is_alphabetic() {
        s.to_camel_case()
    } else {
        format!("V{}", s)
    }
}

fn is_copy(ty: &TypeRef) -> bool {
    match &**ty.type_() {
        Type::Record(r) => r.members.iter().all(|t| is_copy(&t.tref)),
        Type::Variant(v) => v.cases.iter().filter_map(|v| v.tref.as_ref()).all(is_copy),
        Type::Handle(_) | Type::List(_) => false,

        Type::Builtin(BuiltinType::Char)
        | Type::Builtin(BuiltinType::U8 { .. })
        | Type::Builtin(BuiltinType::S8)
        | Type::Builtin(BuiltinType::U16)
        | Type::Builtin(BuiltinType::S16)
        | Type::Builtin(BuiltinType::U32 { .. })
        | Type::Builtin(BuiltinType::S32)
        | Type::Builtin(BuiltinType::U64)
        | Type::Builtin(BuiltinType::S64)
        | Type::Builtin(BuiltinType::F32)
        | Type::Builtin(BuiltinType::F64)
        | Type::Pointer(_)
        | Type::ConstPointer(_) => true,
    }
}

fn is_clone(ty: &TypeRef) -> bool {
    match &**ty.type_() {
        Type::Record(r) => r.members.iter().all(|t| is_clone(&t.tref)),
        Type::Variant(v) => v.cases.iter().filter_map(|v| v.tref.as_ref()).all(is_clone),
        Type::Handle(_) => false,
        Type::List(t) => is_clone(t),
        Type::Builtin(_) | Type::Pointer(_) | Type::ConstPointer(_) => true,
    }
}

impl TypeInfo {
    fn param_name(&self, name: &Id) -> String {
        let name = name.as_str().to_camel_case();
        if self.result && self.owns_data {
            format!("{}Param", name)
        } else {
            name
        }
    }

    fn result_name(&self, name: &Id) -> String {
        let name = name.as_str().to_camel_case();
        if self.param && self.owns_data {
            format!("{}Result", name)
        } else {
            name
        }
    }
}
