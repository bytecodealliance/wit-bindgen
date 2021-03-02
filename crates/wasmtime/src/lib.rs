use heck::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_core::{witx::*, Files, Generator, TypeInfo, Types};
use witx_bindgen_rust_core::{int_repr, to_rust_ident, TypeInfoExt, TypeMode, TypePrint};

#[derive(Default)]
pub struct Wasmtime {
    tmp: usize,
    src: String,
    opts: Opts,
    needs_mem: bool,
    needs_fmt: bool,
    needs_memory: bool,
    needs_get_memory: bool,
    needs_char_from_i32: bool,
    needs_invalid_variant: bool,
    needs_validate_flags: bool,
    needs_store: bool,
    needs_load: bool,
    needs_bad_int: bool,
    types: Types,
    imports: HashMap<Id, Vec<Import>>,
}

pub struct Import {
    name: String,
    trait_signature: String,
    closure: String,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "structopt", structopt(long))]
    rustfmt: bool,
}

impl Opts {
    pub fn build(self) -> Wasmtime {
        let mut r = Wasmtime::new();
        r.opts = self;
        r
    }
}

impl Wasmtime {
    pub fn new() -> Wasmtime {
        Wasmtime::default()
    }

    fn modes_of(&self, ty: &Id) -> Vec<(String, TypeMode)> {
        let info = self.types.get(ty);
        let mut result = Vec::new();
        if info.owns_data() {
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
}

impl TypePrint for Wasmtime {
    fn tmp(&mut self) -> usize {
        let ret = self.tmp;
        self.tmp += 1;
        ret
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: &Id) -> TypeInfo {
        self.types.get(ty)
    }

    fn print_usize(&mut self) {
        self.src.push_str("u32");
    }

    fn print_pointer(&mut self, const_: bool, ty: &TypeRef) {
        self.push_str("*");
        if const_ {
            self.push_str("const ");
        } else {
            self.push_str("mut ");
        }
        match &**ty.type_() {
            Type::Builtin(_) | Type::Pointer(_) | Type::ConstPointer(_) => {
                self.print_tref(ty, TypeMode::Owned);
            }
            Type::List(_) | Type::Variant(_) => panic!("unsupported type"),
            Type::Handle(_) | Type::Record(_) => {
                self.needs_mem = true;
                self.push_str("mem::ManuallyDrop<");
                self.print_tref(ty, TypeMode::Owned);
                self.push_str(">");
            }
        }
    }

    fn print_borrowed_slice(&mut self, ty: &TypeRef, lifetime: &'static str) {
        self.push_str("GuestPtr<");
        self.push_str(lifetime);
        self.push_str(",[");
        self.print_tref(ty, TypeMode::Lifetime(lifetime));
        self.push_str("]>");
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("GuestPtr<");
        self.push_str(lifetime);
        self.push_str(",str>");
    }
}

impl Generator for Wasmtime {
    fn preprocess(&mut self, doc: &Document) {
        self.types.analyze(doc);
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
        let info = self.types.get(name);

        if let Some(repr) = record.bitflags_repr() {
            let name = name.as_str();
            self.src.push_str("bitflags::bitflags! {\n");
            self.rustdoc(docs);
            self.src
                .push_str(&format!("pub struct {}: ", name.to_camel_case()));
            self.int_repr(repr);
            self.src.push_str(" {\n");
            for (i, member) in record.members.iter().enumerate() {
                self.rustdoc(&member.docs);
                self.src.push_str(&format!(
                    "const {}: {} = 1 << {};\n",
                    member.name.as_str().to_shouty_snake_case(),
                    name.to_camel_case(),
                    i,
                ));
            }
            self.src.push_str("}\n");
            self.src.push_str("}\n\n");

            self.src.push_str("impl fmt::Display for ");
            self.src.push_str(&name.to_camel_case());
            self.src
                .push_str("{\nfn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n");

            self.src.push_str("f.write_str(\"");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str("(\")?;\n");
            self.src.push_str("std::fmt::Debug::fmt(self, f)?;\n");
            self.src.push_str("f.write_str(\" (0x\")?;\n");
            self.src
                .push_str("std::fmt::LowerHex::fmt(&self.bits, f)?;\n");
            self.src.push_str("f.write_str(\"))\")?;\n");
            self.src.push_str("Ok(())");

            self.src.push_str("}\n");
            self.src.push_str("}\n\n");
            return;
        }
        for (name, mode) in self.modes_of(name) {
            if !info.has_handle {
                if !info.owns_data() {
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
                self.print_tref(&member.tref, mode);
                self.src.push_str(",\n");
            }
            self.src.push_str("}\n");
        }
    }

    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str) {
        // TODO: should this perhaps be an attribute in the witx file?
        let is_error = name.as_str().contains("errno") && variant.is_enum();
        let info = self.types.get(name);

        for (name, mode) in self.modes_of(name) {
            self.rustdoc(docs);
            if !info.has_handle {
                if variant.is_enum() {
                    self.src.push_str("#[repr(");
                    self.int_repr(variant.tag_repr);
                    self.src.push_str(")]\n#[derive(Copy, PartialEq, Eq)]\n");
                } else if !info.owns_data() {
                    self.src.push_str("#[derive(Copy)]\n");
                }
                self.src.push_str("#[derive(Clone)]\n");
            }
            if !is_error {
                self.src.push_str("#[derive(Debug)]\n");
            }
            self.src
                .push_str(&format!("pub enum {} {{\n", name.to_camel_case()));
            for case in variant.cases.iter() {
                self.rustdoc(&case.docs);
                self.src.push_str(&case_name(&case.name));
                if let Some(ty) = &case.tref {
                    self.src.push_str("(");
                    self.print_tref(ty, mode);
                    self.src.push_str(")")
                }
                self.src.push_str(",\n");
            }
            self.src.push_str("}\n");

            // Auto-synthesize an implementation of the standard `Error` trait for
            // error-looking types based on their name.
            if is_error {
                self.needs_fmt = true;
                self.src.push_str("impl ");
                self.src.push_str(&name);
                self.src.push_str("{\n");

                self.src.push_str("pub fn name(&self) -> &'static str {\n");
                self.src.push_str("match self {");
                for case in variant.cases.iter() {
                    self.src.push_str(&name);
                    self.src.push_str("::");
                    self.src.push_str(&case_name(&case.name));
                    self.src.push_str(" => \"");
                    self.src.push_str(case.name.as_str());
                    self.src.push_str("\",");
                }
                self.src.push_str("}\n");
                self.src.push_str("}\n");

                self.src
                    .push_str("pub fn message(&self) -> &'static str {\n");
                self.src.push_str("match self {");
                for case in variant.cases.iter() {
                    self.src.push_str(&name);
                    self.src.push_str("::");
                    self.src.push_str(&case_name(&case.name));
                    self.src.push_str(" => \"");
                    self.src.push_str(case.docs.trim());
                    self.src.push_str("\",");
                }
                self.src.push_str("}\n");
                self.src.push_str("}\n");

                self.src.push_str("}\n");

                self.src.push_str("impl fmt::Debug for ");
                self.src.push_str(&name);
                self.src
                    .push_str("{\nfn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n");
                self.src.push_str("f.debug_struct(\"");
                self.src.push_str(&name);
                self.src.push_str("\")");
                self.src.push_str(".field(\"code\", &(*self as i32))");
                self.src.push_str(".field(\"name\", &self.name())");
                self.src.push_str(".field(\"message\", &self.message())");
                self.src.push_str(".finish()");
                self.src.push_str("}\n");
                self.src.push_str("}\n");

                self.src.push_str("impl fmt::Display for ");
                self.src.push_str(&name);
                self.src
                    .push_str("{\nfn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n");
                self.src
                    .push_str("write!(f, \"{} (error {})\", self.name(), *self as i32)");
                self.src.push_str("}\n");
                self.src.push_str("}\n");
                self.src.push_str("\n");
                self.src.push_str("impl std::error::Error for ");
                self.src.push_str(&name);
                self.src.push_str("{}\n");
            }
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
            self.print_lifetime_param(mode);
            self.src.push_str(" = ");
            self.print_list(ty, mode);
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
        self.print_tref(ty, TypeMode::Owned);
        self.src.push(';');
    }

    fn type_builtin(&mut self, name: &Id, ty: BuiltinType, docs: &str) {
        self.rustdoc(docs);
        self.src
            .push_str(&format!("pub type {}", name.as_str().to_camel_case()));
        self.src.push_str(" = ");
        self.print_builtin(ty);
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
        let prev = mem::take(&mut self.src);

        let rust_name = func.name.as_str().to_snake_case();
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        self.rustdoc_params(&func.results, "Return");
        self.push_str("fn ");
        self.push_str(to_rust_ident(&rust_name));

        self.push_str("(&self,");
        for param in func.params.iter() {
            self.push_str(to_rust_ident(param.name.as_str()));
            self.push_str(": ");
            self.print_tref(&param.tref, TypeMode::Borrowed("'_"));
            self.push_str(",");
        }
        self.push_str(")");

        match func.results.len() {
            0 => {}
            1 => {
                self.push_str(" -> ");
                self.print_tref(&func.results[0].tref, TypeMode::Owned);
            }
            _ => {
                self.push_str(" -> (");
                for result in func.results.iter() {
                    self.print_tref(&result.tref, TypeMode::Owned);
                    self.push_str(", ");
                }
                self.push_str(")");
            }
        }
        let trait_signature = mem::take(&mut self.src);

        let mut params = Vec::new();
        let sig = func.wasm_signature();
        self.src.push_str("move |_caller: wasmtime::Caller<'_>");
        for (i, param) in sig.params.iter().enumerate() {
            let arg = format!("arg{}", i);
            self.src.push_str(",");
            self.src.push_str(&arg);
            self.src.push_str(":");
            self.wasm_type(*param);
            params.push(arg);
        }
        self.src.push_str("| -> Result<_, wasmtime::Trap> {\n");
        let pos = self.src.len();
        func.call(
            module,
            CallMode::DefinedImport,
            &mut WasmtimeBindgen {
                cfg: self,
                params,
                block_storage: Vec::new(),
                blocks: Vec::new(),
            },
        );
        self.src.push_str("}");

        if mem::take(&mut self.needs_memory) {
            self.src
                .insert_str(pos, "let memory = get_memory(&_caller, \"memory\")?;\n");
            self.needs_memory = false;
            self.needs_get_memory = true;
        }

        let closure = mem::replace(&mut self.src, prev);
        self.imports
            .entry(module.clone())
            .or_insert(Vec::new())
            .push(Import {
                name: func.name.as_str().to_string(),
                closure,
                trait_signature,
            });
    }

    fn export(&mut self, module: &Id, func: &InterfaceFunc) {
        drop((module, func));
        unimplemented!()
    }

    fn finish(&mut self) -> Files {
        let mut files = Files::default();

        let mut src = mem::take(&mut self.src);

        if self.needs_mem {
            src.insert_str(0, "use std::mem;\n");
        }
        if self.needs_fmt {
            src.insert_str(0, "use std::fmt;\n");
        }
        if self.imports.len() > 0 {
            src.insert_str(0, "use anyhow::Result;\n");
        }

        for (module, funcs) in self.imports.iter() {
            src.push_str("\npub trait ");
            src.push_str(&module.as_str().to_camel_case());
            src.push_str("{\n");
            for f in funcs {
                src.push_str(&f.trait_signature);
                src.push_str(";\n\n");
            }
            src.push_str("}\n");
        }

        for (module, funcs) in self.imports.iter() {
            src.push_str("\npub fn add_");
            src.push_str(module.as_str());
            src.push_str("_to_linker(module: impl ");
            src.push_str(&module.as_str().to_camel_case());
            src.push_str(", linker: &mut wasmtime::Linker) -> Result<()> {\n");
            src.push_str("let module = std::rc::Rc::new(module);\n");
            if self.needs_store {
                src.push_str(
                    "
                        fn store(mem: &wasmtime::Memory, offset: u32, bytes: &[u8]) -> Result<(), wasmtime::Trap> {
                            mem.write(offset as usize, bytes)?;
                            Ok(())
                        }
                    ",
                );
            }
            if self.needs_load {
                src.push_str(
                    "
                        fn load<T: AsMut<[u8]>, U>(
                            mem: &wasmtime::Memory,
                            offset: u32,
                            mut bytes: T,
                            cvt: impl FnOnce(T) -> U,
                        ) -> Result<U, wasmtime::Trap> {
                            mem.read(offset as usize, bytes.as_mut())?;
                            Ok(cvt(bytes))
                        }
                    ",
                );
            }
            if self.needs_get_memory {
                src.push_str(
                    "
                        fn get_memory(
                            caller: &wasmtime::Caller<'_>,
                            mem: &str,
                        ) -> Result<wasmtime::Memory, wasmtime::Trap> {
                            let mem = caller.get_export(mem)
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not available\", mem);
                                    wasmtime::Trap::new(msg)
                                })?
                                .into_memory()
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not a memory\", mem);
                                    wasmtime::Trap::new(msg)
                                })?;
                            Ok(mem)
                        }
                    ",
                );
            }
            if self.needs_char_from_i32 {
                src.push_str(
                    "
                        fn char_from_i32(
                            val: i32,
                        ) -> Result<char, wasmtime::Trap> {
                            std::char::from_u32(val as u32)
                                .ok_or_else(|| {
                                    wasmtime::Trap::new(\"char value out of valid range\")
                                })
                        }
                    ",
                );
            }
            if self.needs_invalid_variant {
                src.push_str(
                    "
                        fn invalid_variant(name: &str) -> wasmtime::Trap {
                            let msg = format!(\"invalid discriminant for `{}`\", name);
                            wasmtime::Trap::new(msg)
                        }
                    ",
                );
            }
            if self.needs_bad_int {
                src.push_str(
                    "
                        fn bad_int(_: std::num::TryFromIntError) -> wasmtime::Trap {
                            let msg = \"out-of-bounds integer conversion\";
                            wasmtime::Trap::new(msg)
                        }
                    ",
                );
            }
            if self.needs_validate_flags {
                src.push_str(
                    "
                        fn validate_flags<U>(
                            bits: i64,
                            all: i64,
                            name: &str,
                            mk: impl FnOnce(i64) -> U,
                        ) -> Result<U, wasmtime::Trap> {
                            if bits & !all != 0 {
                                let msg = format!(\"invalid flags specified for `{}`\", name);
                                Err(wasmtime::Trap::new(msg))
                            } else {
                                Ok(mk(bits))
                            }
                        }
                    ",
                );
            }

            for f in funcs {
                src.push_str("let m = module.clone();\n");
                src.push_str(&format!(
                    "linker.func(\"{}\", \"{}\", {});\n",
                    module.as_str(),
                    f.name,
                    f.closure,
                ));
            }
            src.push_str("}\n");
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

struct WasmtimeBindgen<'a> {
    cfg: &'a mut Wasmtime,
    params: Vec<String>,
    block_storage: Vec<String>,
    blocks: Vec<String>,
}

impl WasmtimeBindgen<'_> {
    fn push_str(&mut self, s: &str) {
        self.cfg.src.push_str(s);
    }
}

impl Bindgen for WasmtimeBindgen<'_> {
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
        let tmp = self.cfg.tmp();
        self.cfg.needs_mem = true;
        self.push_str(&format!("let mut rp{} = mem::MaybeUninit::<", tmp));
        self.push_str(&ty.name.as_str().to_camel_case());
        self.push_str(">::uninit();");
        self.push_str(&format!("let ptr{} = rp{0}.as_mut_ptr() as i32;\n", tmp));
        format!("ptr{}", tmp)
    }

    fn allocate_i64_array(&mut self, amt: usize) -> String {
        let tmp = self.cfg.tmp();
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
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
        };

        let cfg = &mut self.cfg;
        let mut try_from = |cvt: &str, operands: &[String], results: &mut Vec<String>| {
            cfg.needs_bad_int = true;
            let result = format!("{}::try_from({}).ok_or_else(bad_int)", cvt, operands[0]);
            results.push(result);
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

            // Downcasts from `i32` into smaller integers are checked to ensure
            // that they fit within the valid range. While not strictly
            // necessary since we could chop bits off this should be more
            // forward-compatible with any future changes.
            Instruction::S8FromI32 => try_from("i8", operands, results),
            Instruction::Char8FromI32 | Instruction::U8FromI32 => try_from("u8", operands, results),
            Instruction::S16FromI32 => try_from("i16", operands, results),
            Instruction::U16FromI32 => try_from("u16", operands, results),

            // Casts of the same bit width simply use `as` since we're just
            // reinterpreting the bits already there.
            Instruction::U32FromI32 | Instruction::UsizeFromI32 => top_as("u32"),
            Instruction::U64FromI64 => top_as("u64"),

            Instruction::CharFromI32 => {
                self.cfg.needs_char_from_i32 = true;
                results.push(format!("char_from_i32({})?", operands[0]));
            }

            Instruction::Bitcasts { casts } => {
                witx_bindgen_rust_core::bitcast(casts, operands, results)
            }

            Instruction::I32FromOwnedHandle { .. } => {
                results.push("ZZZ".to_string());
            }
            Instruction::I32FromBorrowedHandle { .. } => unimplemented!(),
            Instruction::HandleBorrowedFromI32 { .. } => {
                results.push("YYY".to_string());
            }
            Instruction::HandleOwnedFromI32 { .. } => unimplemented!(),

            Instruction::I32FromBitflags { .. } => {
                results.push(format!("({}).bits as i32", operands[0]));
            }
            Instruction::I64FromBitflags { .. } => {
                results.push(format!("({}).bits as i64", operands[0]));
            }
            Instruction::BitflagsFromI32 { repr, name, .. }
            | Instruction::BitflagsFromI64 { repr, name, .. } => {
                self.cfg.needs_validate_flags = true;
                results.push(format!(
                    "validate_flags(
                        i64::from({}),
                        {name}::all().bits() as i64,
                        \"{name}\",
                        |b| {name} {{ bits: b as {ty} }}
                    )?",
                    operands[0],
                    name = name.name.as_str().to_camel_case(),
                    ty = int_repr(*repr),
                ));
            }

            Instruction::RecordLower { ty, name } => {
                let tmp = self.cfg.tmp();
                self.cfg.record_lower(ty, *name, tmp, &operands[0], results);
            }
            Instruction::RecordLift { ty, name } => {
                self.cfg.record_lift(ty, *name, operands, results);
            }

            Instruction::VariantPayload => results.push("e".to_string()),

            Instruction::VariantLower { ty, name, nresults } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                self.cfg
                    .variant_lower(ty, *name, *nresults, &operands[0], results, blocks);
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
                    result.push_str(&i.to_string());
                    result.push_str(" => ");
                    self.cfg
                        .variant_lift_case(ty, *name, case, &block, &mut result);
                    result.push_str(",\n");
                }
                let variant_name = name.map(|s| s.name.as_str().to_camel_case());
                let variant_name = variant_name.as_deref().unwrap_or_else(|| {
                    if ty.is_bool() {
                        "bool"
                    } else if ty.as_expected().is_some() {
                        "Result"
                    } else if ty.as_option().is_some() {
                        "Option"
                    } else {
                        unimplemented!()
                    }
                });
                result.push_str("_ => return Err(invalid_variant(\"");
                result.push_str(&variant_name);
                result.push_str("\")),\n");
                result.push_str("}");
                results.push(result);
                self.cfg.needs_invalid_variant = true;
            }

            Instruction::ListCanonLower { element, malloc } => {
                let tmp = self.cfg.tmp();
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
                let tmp = self.cfg.tmp();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {} as usize;\n", len, operands[1]));
                let result = format!(
                    "Vec::from_raw_parts({} as *mut _, {1}, {1})",
                    operands[0], len
                );
                match &**element.type_() {
                    Type::Builtin(BuiltinType::Char) => {
                        results.push(format!("String::from_utf8({}).unwrap_or_else(XXX)", result));
                    }
                    _ => results.push(result),
                }
            }

            Instruction::ListLower { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.cfg.tmp();
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
                let tmp = self.cfg.tmp();
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
                drop((module, name, params, func_results));
                unimplemented!()
            }

            Instruction::CallInterface { module: _, func } => {
                self.cfg.let_results(func.results.len(), results);
                self.push_str("m.");
                self.push_str(func.name.as_str());
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
            }

            Instruction::Return { amt: 0 } => {
                self.push_str("Ok(())");
            }
            Instruction::Return { amt: 1 } => {
                self.push_str("Ok(");
                self.push_str(&operands[0]);
                self.push_str(")");
            }
            Instruction::Return { .. } => {
                self.push_str("Ok((");
                self.push_str(&operands.join(", "));
                self.push_str("))");
            }

            Instruction::I32Load { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "load(&memory, {} + {}, [0u8; 4], i32::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load8U { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "i32::from(load(&memory, {} + {}, [0u8; 1], u8::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load8S { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "i32::from(load(&memory, {} + {}, [0u8; 1], i8::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load16U { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "i32::from(load(&memory, {} + {}, [0u8; 2], u16::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I32Load16S { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "i32::from(load(&memory, {} + {}, [0u8; 2], i16::from_le_bytes)?)",
                    operands[0], offset,
                ));
            }
            Instruction::I64Load { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "load(&memory, {} + {}, [0u8; 8], i64::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::F32Load { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "load(&memory, {} + {}, [0u8; 4], f32::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::F64Load { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_load = true;
                results.push(format!(
                    "load(&memory, {} + {}, [0u8; 8], f64::from_le_bytes)?",
                    operands[0], offset,
                ));
            }
            Instruction::I32Store { offset }
            | Instruction::I64Store { offset }
            | Instruction::F32Store { offset }
            | Instruction::F64Store { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_store = true;
                self.push_str(&format!(
                    "store(&memory, ({} + {}) as u32, &({}).into_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_store = true;
                self.push_str(&format!(
                    "store(&memory, ({} + {}) as u32, &(({}) as u8).into_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_store = true;
                self.push_str(&format!(
                    "store(&memory, ({} + {}) as u32, &(({}) as u16).into_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }

            Instruction::Witx { instr } => match instr {
                WitxInstruction::PointerFromI32 { .. }
                | WitxInstruction::ConstPointerFromI32 { .. } => {
                    for _ in 0..instr.results_len() {
                        results.push("XXX".to_string());
                    }
                }
                i => unimplemented!("{:?}", i),
            },
        }
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
