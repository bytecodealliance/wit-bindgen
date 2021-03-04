use heck::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_gen_core::{witx::*, Files, Generator, TypeInfo, Types};
use witx_bindgen_gen_rust::{int_repr, to_rust_ident, TypeMode, TypePrint};

#[derive(Default)]
pub struct Wasmtime {
    tmp: usize,
    src: String,
    opts: Opts,
    needs_memory: bool,
    needs_guest_memory: bool,
    needs_get_memory: bool,
    needs_get_func: bool,
    needs_char_from_i32: bool,
    needs_invalid_variant: bool,
    needs_validate_flags: bool,
    needs_store: bool,
    needs_load: bool,
    needs_bad_int: bool,
    needs_borrow_checker: bool,
    needs_slice_as_bytes: bool,
    needs_functions: HashMap<String, NeededFunction>,
    types: Types,
    imports: HashMap<Id, Vec<Import>>,
}

enum NeededFunction {
    Malloc,
    Free,
}

struct Import {
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
}

impl TypePrint for Wasmtime {
    fn is_host(&self) -> bool {
        true
    }

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
                self.push_str("core::mem::ManuallyDrop<");
                self.print_tref(ty, TypeMode::Owned);
                self.push_str(">");
            }
        }
    }

    fn print_borrowed_slice(&mut self, ty: &TypeRef, lifetime: &'static str) {
        self.push_str("witx_bindgen_wasmtime::GuestPtr<");
        self.push_str(lifetime);
        self.push_str(",[");
        // This should only ever be used on types without lifetimes, so use
        // invalid syntax here to catch bugs where that's not the case.
        self.print_tref(ty, TypeMode::Lifetime("INVALID"));
        self.push_str("]>");
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("witx_bindgen_wasmtime::GuestPtr<");
        self.push_str(lifetime);
        self.push_str(",str>");
    }
}

impl Generator for Wasmtime {
    fn preprocess(&mut self, doc: &Document) {
        self.types.analyze(doc);
    }

    fn type_record(&mut self, name: &Id, record: &RecordDatatype, docs: &str) {
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
                    "const {} = 1 << {};\n",
                    member.name.as_str().to_camel_case(),
                    i
                ));
            }
            self.src.push_str("}\n");
            self.src.push_str("}\n\n");

            self.src.push_str("impl core::fmt::Display for ");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str(
                "{\nfn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
            );

            self.src.push_str("f.write_str(\"");
            self.src.push_str(&name.to_camel_case());
            self.src.push_str("(\")?;\n");
            self.src.push_str("core::fmt::Debug::fmt(self, f)?;\n");
            self.src.push_str("f.write_str(\" (0x\")?;\n");
            self.src
                .push_str("core::fmt::LowerHex::fmt(&self.bits, f)?;\n");
            self.src.push_str("f.write_str(\"))\")?;\n");
            self.src.push_str("Ok(())");

            self.src.push_str("}\n");
            self.src.push_str("}\n\n");
            return;
        }

        self.print_typedef_record(name, record, docs);
    }

    fn type_variant(&mut self, name: &Id, variant: &Variant, docs: &str) {
        self.print_typedef_variant(name, variant, docs);
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
        self.src.push_str(
            " {
                pub unsafe fn from_raw(raw: i32) -> Self {
                    Self(raw)
                }

                pub fn into_raw(self) -> i32 {
                    let ret = self.0;
                    core::mem::forget(self);
                    return ret;
                }
            }",
        );
    }

    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str) {
        self.print_typedef_alias(name, ty, docs);
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
            self.print_tref(&param.tref, TypeMode::LeafBorrowed("'_"));
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

        if self.needs_guest_memory {
            // TODO: this unsafe isn't justified and it's actually unsafe, we
            // need a better solution for where to store this.
            self.src.insert_str(
                pos,
                "let guest_memory = unsafe { witx_bindgen_wasmtime::WasmtimeGuestMemory::new(
                    &memory,
                    m.borrow_checker(),
                ) };\n",
            );
            self.needs_borrow_checker = true;
        }
        if self.needs_memory || self.needs_guest_memory {
            self.src
                .insert_str(pos, "let memory = get_memory(&_caller, \"memory\")?;\n");
            self.needs_get_memory = true;
        }

        self.needs_memory = false;
        self.needs_guest_memory = false;

        for (name, func) in self.needs_functions.drain() {
            self.src.insert_str(
                pos,
                &format!(
                    "
                        let func = get_func(&_caller, \"{name}\")?;
                        let func_{name} = func.get{cvt}()?;
                    ",
                    name = name,
                    cvt = match func {
                        NeededFunction::Malloc => "1::<i32, i32>",
                        NeededFunction::Free => "2::<i32, i32, ()>",
                    },
                ),
            );
            self.needs_get_func = true;
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

        if self.imports.len() > 0 {
            src.insert_str(0, "use anyhow::Result;\n");
        }

        for (module, funcs) in self.imports.iter() {
            src.push_str("\npub trait ");
            src.push_str(&module.as_str().to_camel_case());
            src.push_str("{\n");
            if self.needs_borrow_checker {
                src.push_str(
                    "fn borrow_checker(&self) -> &witx_bindgen_wasmtime::BorrowChecker;\n",
                );
            }
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
            src.push_str(" + 'static, linker: &mut wasmtime::Linker) -> Result<()> {\n");
            src.push_str("let module = std::rc::Rc::new(module);\n");
            if self.needs_store {
                src.push_str(
                    "
                        fn store(mem: &wasmtime::Memory, offset: i32, bytes: &[u8]) -> Result<(), wasmtime::Trap> {
                            unsafe {
                                mem.data_unchecked_mut()
                                    .get_mut(offset as usize..)
                                    .and_then(|s| s.get_mut(..bytes.len()))
                                    .ok_or_else(|| wasmtime::Trap::new(\"out of bounds write\"))?
                                    .copy_from_slice(bytes);
                            }
                            //mem.write(offset as usize, bytes)?;
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
                            offset: i32,
                            mut bytes: T,
                            cvt: impl FnOnce(T) -> U,
                        ) -> Result<U, wasmtime::Trap> {
                            unsafe {
                                let slice = mem.data_unchecked_mut()
                                    .get(offset as usize..)
                                    .and_then(|s| s.get(..bytes.as_mut().len()))
                                    .ok_or_else(|| wasmtime::Trap::new(\"out of bounds read\"))?;
                                bytes.as_mut().copy_from_slice(slice);
                            }
                            //mem.read(offset as usize, bytes.as_mut())?;
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
            if self.needs_get_func {
                src.push_str(
                    "
                        fn get_func(
                            caller: &wasmtime::Caller<'_>,
                            func: &str,
                        ) -> Result<wasmtime::Func, wasmtime::Trap> {
                            let func = caller.get_export(func)
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not available\", func);
                                    wasmtime::Trap::new(msg)
                                })?
                                .into_func()
                                .ok_or_else(|| {
                                    let msg = format!(\"`{}` export not a function\", func);
                                    wasmtime::Trap::new(msg)
                                })?;
                            Ok(func)
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
                            core::char::from_u32(val as u32)
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
                src.push_str("use core::convert::TryFrom;\n");
                src.push_str(
                    "
                        fn bad_int(_: core::num::TryFromIntError) -> wasmtime::Trap {
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
            if self.needs_slice_as_bytes {
                src.push_str(
                    "
                        unsafe fn slice_as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
                            core::slice::from_raw_parts(
                                slice.as_ptr() as *const u8,
                                core::mem::size_of_val(slice),
                            )
                        }
                    ",
                );
            }

            for f in funcs {
                src.push_str("let m = module.clone();\n");
                src.push_str(&format!(
                    "linker.func(\"{}\", \"{}\", {})?;\n",
                    module.as_str(),
                    f.name,
                    f.closure,
                ));
            }
            src.push_str("Ok(())\n}\n");
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
        self.push_str(&format!("let mut rp{} = core::mem::MaybeUninit::<", tmp));
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
            let result = format!("{}::try_from({}).map_err(bad_int)?", cvt, operands[0]);
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
                witx_bindgen_gen_rust::bitcast(casts, operands, results)
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
                // Lowering only happens when we're passing lists into wasm,
                // which forces us to always allocate, so this should always be
                // `Some`.
                let malloc = malloc.as_ref().unwrap();
                self.cfg
                    .needs_functions
                    .insert(malloc.clone(), NeededFunction::Malloc);
                let size_align = element.mem_size_align();

                // Store the operand into a temporary...
                let tmp = self.cfg.tmp();
                let val = format!("vec{}", tmp);
                self.push_str(&format!("let {} = {};\n", val, operands[0]));

                // ... and then malloc space for the result in the guest module
                let ptr = format!("ptr{}", tmp);
                self.push_str(&format!(
                    "let {} = func_{}(({}.len() as i32) * {})?;\n",
                    ptr, malloc, val, size_align.size
                ));

                // ... and then copy over the result.
                //
                // Note the unsafety here, in general it's not safe to copy
                // from arbitrary types on the host as a slice of bytes, but in
                // this case we should be able to get away with it since
                // canonical lowerings have the same memory representation on
                // the host as in the guest.
                self.push_str(&format!(
                    "store(&memory, {}, unsafe {{ slice_as_bytes({}.as_ref()) }})?;\n",
                    ptr, val
                ));
                self.cfg.needs_store = true;
                self.cfg.needs_slice_as_bytes = true;
                results.push(ptr);
                results.push(format!("{}.len() as i32", val));
            }

            Instruction::ListCanonLift { element: _, free } => {
                assert!(free.is_none());
                self.cfg.needs_guest_memory = true;
                // Note the unsafety here. This is possibly an unsafe operation
                // because the representation of the target must match the
                // representation on the host, but `ListCanonLift` is only
                // generated for types where that's true, so this should be
                // safe.
                results.push(format!(
                    "
                        unsafe {{
                            witx_bindgen_wasmtime::GuestPtr::new(
                                &guest_memory,
                                (({}) as u32, ({}) as u32),
                            )
                        }}
                    ",
                    operands[0], operands[1]
                ));
            }

            Instruction::ListLower {
                element,
                owned,
                malloc,
            } => {
                assert!(*owned);
                let body = self.blocks.pop().unwrap();
                let tmp = self.cfg.tmp();
                let vec = format!("vec{}", tmp);
                let result = format!("result{}", tmp);
                let len = format!("len{}", tmp);
                self.cfg
                    .needs_functions
                    .insert(malloc.clone(), NeededFunction::Malloc);
                let size_align = element.mem_size_align();

                // first store our vec-to-lower in a temporary since we'll
                // reference it multiple times.
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));

                // ... then malloc space for the result in the guest module
                self.push_str(&format!(
                    "let {} = func_{}({} * {})?;\n",
                    result, malloc, len, size_align.size
                ));

                // ... then consume the vector and use the block to lower the
                // result.
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!(
                    "let base = {} + (i as i32) * {};\n",
                    result, size_align.size,
                ));
                self.push_str(&body);
                self.push_str("}");

                results.push(result);
                results.push(len);
            }

            Instruction::ListLift { element, free } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.cfg.tmp();
                let size_align = element.mem_size_align();
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {};\n", len, operands[1]));
                let base = format!("base{}", tmp);
                self.push_str(&format!("let {} = {};\n", base, operands[0]));
                let result = format!("result{}", tmp);
                self.push_str(&format!(
                    "let mut {} = Vec::with_capacity({} as usize);\n",
                    result, len,
                ));

                self.push_str("for i in 0..");
                self.push_str(&len);
                self.push_str(" {\n");
                self.push_str("let base = ");
                self.push_str(&base);
                self.push_str(" + i *");
                self.push_str(&size_align.size.to_string());
                self.push_str(";\n");
                self.push_str(&result);
                self.push_str(".push(");
                self.push_str(&body);
                self.push_str(");\n");
                self.push_str("}\n");
                self.push_str(&format!("func_{}({}, {})?;\n", free, base, len));
                results.push(result);
                self.cfg
                    .needs_functions
                    .insert(free.to_string(), NeededFunction::Free);
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
                    "store(&memory, {} + {}, &({}).to_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store8 { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_store = true;
                self.push_str(&format!(
                    "store(&memory, {} + {}, &(({}) as u8).to_le_bytes())?;\n",
                    operands[1], offset, operands[0]
                ));
            }
            Instruction::I32Store16 { offset } => {
                self.cfg.needs_memory = true;
                self.cfg.needs_store = true;
                self.push_str(&format!(
                    "store(&memory, {} + {}, &(({}) as u16).to_le_bytes())?;\n",
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
