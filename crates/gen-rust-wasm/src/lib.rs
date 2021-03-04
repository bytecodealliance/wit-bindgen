use heck::*;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use witx_bindgen_gen_core::{witx::*, Files, Generator, TypeInfo, Types};
use witx_bindgen_gen_rust::{int_repr, to_rust_ident, wasm_type, TypeMode, TypePrint};

#[derive(Default)]
pub struct RustWasm {
    tmp: usize,
    src: String,
    opts: Opts,
    types: Types,
    params: Vec<String>,
    blocks: Vec<String>,
    block_storage: Vec<String>,
    is_dtor: bool,
    in_import: bool,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "structopt", derive(structopt::StructOpt))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub rustfmt: bool,

    /// Adds the witx module name into import binding names when enabled.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub multi_module: bool,

    /// Whether or not the bindings assume interface values are always
    /// well-formed or whether checks are performed.
    #[cfg_attr(feature = "structopt", structopt(long))]
    pub unchecked: bool,
}

impl Opts {
    pub fn build(self) -> RustWasm {
        let mut r = RustWasm::new();
        r.opts = self;
        r
    }
}

impl RustWasm {
    pub fn new() -> RustWasm {
        RustWasm::default()
    }
}

impl TypePrint for RustWasm {
    fn call_mode(&self) -> CallMode {
        if self.in_import {
            CallMode::DeclaredImport
        } else {
            CallMode::DefinedExport
        }
    }

    fn default_param_mode(&self) -> TypeMode {
        // We default to borrowing as much as possible to maximize the ability
        // for host to take views into our memory without forcing wasm modules
        // to allocate anything.
        TypeMode::AllBorrowed("'a")
    }

    fn handle_projection(&self) -> Option<&'static str> {
        None
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
        self.src.push_str("usize");
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
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str("[");
        self.print_tref(ty, TypeMode::AllBorrowed(lifetime));
        self.push_str("]");
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str(" str");
    }
}

impl Generator for RustWasm {
    fn preprocess(&mut self, doc: &Document) {
        self.types.analyze(doc);
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

                pub fn as_raw(&self) -> i32 {
                    self.0
                }
            }",
        );
        let info = self.info(name);

        if info.handle_with_dtor {
            self.src.push_str("impl Drop for ");
            self.src.push_str(&name.as_str().to_camel_case());
            self.src.push_str(&format!(
                "{{
                    fn drop(&mut self) {{
                        unsafe {{
                            drop({}_close({}(self.0)));
                        }}
                    }}
                }}",
                name.as_str(),
                name.as_str().to_camel_case(),
            ));
        }
    }

    fn type_alias(&mut self, name: &Id, ty: &NamedType, docs: &str) {
        self.print_typedef_alias(name, ty, docs);
    }

    fn type_list(&mut self, name: &Id, ty: &TypeRef, docs: &str) {
        self.print_type_list(name, ty, docs);
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
        self.in_import = true;
        let rust_name = func.name.as_ref().to_snake_case();
        self.rustdoc(&func.docs);
        self.rustdoc_params(&func.params, "Parameters");
        self.rustdoc_params(&func.results, "Return");

        self.is_dtor = self.types.is_dtor_func(&func.name);
        if self.is_dtor {
            self.push_str("unsafe ");
        }
        self.push_str("fn ");
        self.push_str(to_rust_ident(&rust_name));

        self.push_str("(");
        self.params.truncate(0);
        let param_mode = if self.is_dtor {
            TypeMode::Owned
        } else {
            TypeMode::AllBorrowed("'_")
        };
        for param in func.params.iter() {
            self.push_str(to_rust_ident(param.name.as_str()));
            self.params
                .push(to_rust_ident(param.name.as_str()).to_string());
            self.push_str(": ");
            self.print_tref(&param.tref, param_mode);
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
        self.src.push_str("{");
        if !self.is_dtor {
            self.src.push_str("unsafe{");
        }

        func.call(module, CallMode::DeclaredImport, self);

        if !self.is_dtor {
            self.src.push_str("}");
        }
        self.src.push_str("}");
    }

    fn export(&mut self, module: &Id, func: &InterfaceFunc) {
        self.in_import = false;
        let rust_name = func.name.as_ref().to_snake_case();

        self.src.push_str("#[export_name = \"");
        self.src.push_str(&rust_name);
        self.src.push_str("\"]\n");
        self.src
            .push_str("pub unsafe extern \"C\" fn __witx_bindgen_");
        self.src.push_str(&rust_name);
        self.src.push_str("(");
        let sig = func.wasm_signature();
        self.params.truncate(0);
        for (i, param) in sig.params.iter().enumerate() {
            let name = format!("arg{}", i);
            self.src.push_str(&name);
            self.src.push_str(": ");
            self.wasm_type(*param);
            self.src.push_str(",");
            self.params.push(name);
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

        self.is_dtor = false;
        func.call(module, CallMode::DefinedExport, self);

        self.src.push_str("}");
    }

    fn finish(&mut self) -> Files {
        let mut files = Files::default();

        let mut src = mem::take(&mut self.src);

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

impl Bindgen for RustWasm {
    type Operand = String;

    fn push_block(&mut self) {
        let prev_src = mem::take(&mut self.src);
        self.block_storage.push(prev_src);
    }

    fn finish_block(&mut self, operands: &mut Vec<String>) {
        let prev_src = self.block_storage.pop().unwrap();
        let src = mem::replace(&mut self.src, prev_src);
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
        self.push_str(&format!("let mut rp{} = core::mem::MaybeUninit::<", tmp));
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
        let unchecked = self.opts.unchecked;
        let mut top_as = |cvt: &str| {
            let mut s = operands.pop().unwrap();
            s.push_str(" as ");
            s.push_str(cvt);
            results.push(s);
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
                        "core::char::from_u32_unchecked({} as u32)",
                        operands[0]
                    ));
                } else {
                    results.push(format!(
                        "core::char::from_u32({} as u32).unwrap()",
                        operands[0]
                    ));
                }
            }

            Instruction::Bitcasts { casts } => {
                witx_bindgen_gen_rust::bitcast(casts, operands, results)
            }

            Instruction::I32FromOwnedHandle { .. } => unimplemented!(),
            Instruction::I32FromBorrowedHandle { .. } => {
                if self.is_dtor {
                    results.push(format!("{}.into_raw()", operands[0]));
                } else {
                    results.push(format!("{}.0", operands[0]));
                }
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
                self.record_lower(ty, *name, &operands[0], results);
            }
            Instruction::RecordLift { ty, name } => {
                self.record_lift(ty, *name, operands, results);
            }

            Instruction::VariantPayload => results.push("e".to_string()),

            Instruction::VariantLower { ty, name, nresults } => {
                let blocks = self
                    .blocks
                    .drain(self.blocks.len() - ty.cases.len()..)
                    .collect::<Vec<_>>();
                self.variant_lower(ty, *name, *nresults, &operands[0], results, blocks);
            }

            // In unchecked mode when this type is a named enum then we know we
            // defined the type so we can transmute directly into it.
            Instruction::VariantLift {
                ty,
                name: Some(name),
            } if ty.cases.iter().all(|c| c.tref.is_none()) && unchecked => {
                self.blocks.drain(self.blocks.len() - ty.cases.len()..);
                let mut result = format!("core::mem::transmute::<_, ");
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
                    self.variant_lift_case(ty, *name, case, &block, &mut result);
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
                    self.push_str(&format!("core::mem::forget({});\n", val));
                }
                results.push(ptr);
                results.push(len);
            }

            Instruction::ListCanonLift { element, free } => {
                assert!(free.is_some());
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
                let layout = format!("layout{}", tmp);
                let len = format!("len{}", tmp);
                self.push_str(&format!("let {} = {};\n", vec, operands[0]));
                self.push_str(&format!("let {} = {}.len() as i32;\n", len, vec));
                let size_align = element.mem_size_align();
                self.push_str(&format!(
                    "let {} = core::alloc::Layout::from_size_align_unchecked({}.len() * {}, 8);\n",
                    layout, vec, size_align.size,
                ));
                self.push_str(&format!(
                    "let {} = std::alloc::alloc({});\n",
                    result, layout,
                ));
                self.push_str(&format!(
                    "if {}.is_null() {{ std::alloc::handle_alloc_error({}); }}\n",
                    result, layout,
                ));
                self.push_str(&format!(
                    "for (i, e) in {}.into_iter().enumerate() {{\n",
                    vec
                ));
                self.push_str(&format!(
                    "let base = {} as i32 + (i as i32) * {};\n",
                    result, size_align.size,
                ));
                self.push_str(&body);
                self.push_str("}");
                results.push(format!("{} as i32", result));
                results.push(len);
            }

            Instruction::ListLift { element, .. } => {
                let body = self.blocks.pop().unwrap();
                let tmp = self.tmp();
                let size_align = element.mem_size_align();
                let len = format!("len{}", tmp);
                let base = format!("base{}", tmp);
                let result = format!("result{}", tmp);
                self.push_str(&format!("let {} = {};\n", base, operands[0]));
                self.push_str(&format!("let {} = {};\n", len, operands[1],));
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
                results.push(result);
                self.push_str(&format!(
                    "std::alloc::dealloc(
                        {} as *mut _,
                        std::alloc::Layout::from_size_align_unchecked(
                            ({} as usize) * {},
                            8,
                        ),
                    );",
                    base, len, size_align.size
                ));
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
                self.let_results(func.results.len(), results);
                self.push_str(func.name.as_str());
                self.push_str("(");
                self.push_str(&operands.join(", "));
                self.push_str(");");
            }

            Instruction::Return { amt } => match amt {
                0 => {}
                1 => self.push_str(&operands[0]),
                _ => {
                    self.push_str("(");
                    self.push_str(&operands.join(", "));
                    self.push_str(")");
                }
            },

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
                WitxInstruction::AddrOf => {
                    let i = self.tmp();
                    self.push_str(&format!("let t{} = {};\n", i, operands[0]));
                    results.push(format!("&t{} as *const _ as i32", i));
                }
                i => unimplemented!("{:?}", i),
            },
        }
    }
}
