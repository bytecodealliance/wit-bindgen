use witx::{
    Bindgen, BuiltinType, CallMode, Function, Instruction, NamedType, Type, WasmSignature, WasmType,
};

// The parent's memory is imported, so it is always index 0 for the adapter logic
const PARENT_MEMORY_INDEX: u32 = 0;
// The adapted module's memory is aliased, so it is always index 1 for the adapter logic
const ADAPTED_MEMORY_INDEX: u32 = 1;

fn sizeof(ty: &Type) -> usize {
    match ty {
        Type::Record(_) => unimplemented!("support for records is not yet implemented"),
        Type::Variant(_) => unimplemented!("support for variants is not yet implemented"),
        Type::Handle(_) => 4,
        Type::List(l) => sizeof(l.type_()),
        Type::Pointer(_) | Type::ConstPointer(_) | Type::Buffer(_) => 4, // WASM32,
        Type::Builtin(t) => match t {
            BuiltinType::Char | BuiltinType::U8 { .. } | BuiltinType::S8 => 1,
            BuiltinType::U16 | BuiltinType::S16 => 2,
            BuiltinType::U32 { .. } | BuiltinType::S32 | BuiltinType::F32 => 4,
            BuiltinType::U64 | BuiltinType::S64 | BuiltinType::F64 => 8,
        },
    }
}

fn alignment(ty: &Type) -> usize {
    match ty {
        Type::Record(_) => unimplemented!("support for records is not yet implemented"),
        Type::Variant(_) => unimplemented!("support for variants is not yet implemented"),
        Type::Handle(_) => 4,
        Type::List(l) => alignment(l.type_()),
        Type::Pointer(_) | Type::ConstPointer(_) | Type::Buffer(_) => 4, // WASM32,
        Type::Builtin(t) => match t {
            BuiltinType::Char | BuiltinType::U8 { .. } | BuiltinType::S8 => 1,
            BuiltinType::U16 | BuiltinType::S16 => 2,
            BuiltinType::U32 { .. } | BuiltinType::S32 | BuiltinType::F32 => 4,
            BuiltinType::U64 | BuiltinType::S64 | BuiltinType::F64 => 8,
        },
    }
}

fn to_val_type(ty: &WasmType) -> wasm_encoder::ValType {
    match ty {
        WasmType::I32 => wasm_encoder::ValType::I32,
        WasmType::I64 => wasm_encoder::ValType::I64,
        WasmType::F32 => wasm_encoder::ValType::F32,
        WasmType::F64 => wasm_encoder::ValType::F64,
    }
}

enum LoadType {
    I32,
    I32_8U,
    I32_8S,
    I32_16U,
    I32_16S,
    I64,
    F32,
    F64,
}

enum StoreType {
    I32,
    I32_8,
    I32_16,
    I64,
    F32,
    F64,
}

impl From<&WasmType> for StoreType {
    fn from(ty: &WasmType) -> Self {
        match ty {
            WasmType::I32 => Self::I32,
            WasmType::I64 => Self::I64,
            WasmType::F32 => Self::F32,
            WasmType::F64 => Self::F64,
        }
    }
}

#[derive(Debug)]
pub struct CodeGenerator<'a> {
    params: Vec<Operand>,
    signature: WasmSignature,
    locals: Vec<WasmType>,
    instructions: Vec<wasm_encoder::Instruction<'a>>,
    locals_start_index: u32,
    func_index: u32,
    parent_malloc_index: u32,
    malloc_index: u32,
    free_index: u32,
    retptr_alloc: Option<(u32, u32)>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(
        func: &Function,
        func_index: u32,
        parent_malloc_index: u32,
        malloc_index: u32,
        free_index: u32,
    ) -> Self {
        let signature = func.wasm_signature(CallMode::DeclaredExport);

        let mut locals_start_index = 0;
        let params = func
            .params
            .iter()
            .map(|p| {
                let i = locals_start_index;
                locals_start_index += 1;
                match p.tref.type_().as_ref() {
                    Type::Record(_) => unimplemented!(),
                    Type::Variant(_) => unimplemented!(),
                    Type::Handle(_) => Operand::Local(i),
                    Type::List(_) => {
                        locals_start_index += 1;
                        Operand::List {
                            addr: i,
                            len: i + 1,
                        }
                    }
                    Type::Pointer(_) => Operand::Local(i),
                    Type::ConstPointer(_) => Operand::Local(i),
                    Type::Buffer(_) => Operand::Local(i),
                    Type::Builtin(_) => Operand::Local(i),
                }
            })
            .collect();

        // Account for any return pointer in the parameters
        if signature.retptr.is_some() {
            locals_start_index += 1;
        }

        Self {
            params,
            signature,
            locals: Vec::new(),
            instructions: Vec::new(),
            locals_start_index,
            func_index,
            parent_malloc_index,
            malloc_index,
            free_index,
            retptr_alloc: None,
        }
    }

    pub fn into_function(self) -> wasm_encoder::Function {
        let mut function =
            wasm_encoder::Function::new(self.locals.iter().map(|ty| (1, to_val_type(ty))));

        for inst in self.instructions {
            function.instruction(inst);
        }

        function
    }

    fn alloc_local(&mut self, ty: WasmType) -> u32 {
        let index = self.locals_start_index + self.locals.len() as u32;
        self.locals.push(ty);
        index
    }

    fn local_type(&self, index: u32) -> WasmType {
        if index < self.locals_start_index {
            self.signature.params[index as usize]
        } else {
            self.locals[(index - self.locals_start_index) as usize]
        }
    }

    fn emit_load(&mut self, addr: &Operand, offset: u32, ty: LoadType) -> u32 {
        match addr {
            Operand::Pointer { addr, memory } => {
                self.instructions
                    .push(wasm_encoder::Instruction::LocalGet(*addr));

                let memarg = wasm_encoder::MemArg {
                    offset: offset as u32,
                    align: 2,
                    memory_index: *memory,
                };

                let (wasm_ty, inst) = match ty {
                    LoadType::I32 => (WasmType::I32, wasm_encoder::Instruction::I32Load(memarg)),
                    LoadType::I32_8U => {
                        (WasmType::I32, wasm_encoder::Instruction::I32Load8_U(memarg))
                    }
                    LoadType::I32_8S => {
                        (WasmType::I32, wasm_encoder::Instruction::I32Load8_S(memarg))
                    }
                    LoadType::I32_16U => (
                        WasmType::I32,
                        wasm_encoder::Instruction::I32Load16_U(memarg),
                    ),
                    LoadType::I32_16S => (
                        WasmType::I32,
                        wasm_encoder::Instruction::I32Load16_S(memarg),
                    ),
                    LoadType::I64 => (WasmType::I64, wasm_encoder::Instruction::I64Load(memarg)),
                    LoadType::F32 => (WasmType::F32, wasm_encoder::Instruction::F32Load(memarg)),
                    LoadType::F64 => (WasmType::F64, wasm_encoder::Instruction::F64Load(memarg)),
                };

                self.instructions.push(inst);

                let storage = self.alloc_local(wasm_ty);

                self.instructions
                    .push(wasm_encoder::Instruction::LocalSet(storage));

                storage
            }
            _ => panic!("operand must be a pointer"),
        }
    }

    fn emit_store(&mut self, addr: &Operand, offset: u32, operand: &Operand, ty: StoreType) {
        addr.load(&mut self.instructions);
        operand.load(&mut self.instructions);

        match addr {
            Operand::Pointer { memory, .. } => {
                let memarg = wasm_encoder::MemArg {
                    offset: offset as u32,
                    align: 2,
                    memory_index: *memory,
                };

                let inst = match ty {
                    StoreType::I32 => wasm_encoder::Instruction::I32Store(memarg),
                    StoreType::I32_8 => wasm_encoder::Instruction::I32Store8(memarg),
                    StoreType::I32_16 => wasm_encoder::Instruction::I32Store16(memarg),
                    StoreType::I64 => wasm_encoder::Instruction::I64Store(memarg),
                    StoreType::F32 => wasm_encoder::Instruction::F32Store(memarg),
                    StoreType::F64 => wasm_encoder::Instruction::F64Store(memarg),
                };

                self.instructions.push(inst);
            }
            _ => panic!("expected a pointer for first operand"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operand {
    Local(u32),
    I32Const(i32),
    Pointer { addr: u32, memory: u32 },
    List { addr: u32, len: u32 },
}

impl Operand {
    fn local(&self) -> Option<u32> {
        match self {
            Operand::Local(i) => Some(*i),
            _ => None,
        }
    }

    fn load(&self, instructions: &mut Vec<wasm_encoder::Instruction>) {
        match self {
            Self::Local(i) | Self::Pointer { addr: i, .. } => {
                instructions.push(wasm_encoder::Instruction::LocalGet(*i));
            }
            Self::I32Const(i) => {
                instructions.push(wasm_encoder::Instruction::I32Const(*i));
            }
            Self::List { .. } => panic!("list operands must be split"),
        }
    }

    fn split_list(&self) -> (Self, Self) {
        match self {
            Self::List { addr, len } => (Self::Local(*addr), Self::Local(*len)),
            _ => panic!("expected a list operand to split"),
        }
    }
}

impl Bindgen for CodeGenerator<'_> {
    type Operand = Operand;

    fn emit(
        &mut self,
        inst: &Instruction<'_>,
        operands: &mut Vec<Self::Operand>,
        results: &mut Vec<Self::Operand>,
    ) {
        match inst {
            Instruction::GetArg { nth } => {
                results.push(self.params[*nth].clone());
            }
            Instruction::I32Const { val } => {
                results.push(Operand::I32Const(*val));
            }
            Instruction::Bitcasts { .. } => unimplemented!(),
            Instruction::ConstZero { .. } => unimplemented!(),
            Instruction::I32Load { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32,
                )));
            }
            Instruction::I32Load8U { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_8U,
                )));
            }
            Instruction::I32Load8S { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_8S,
                )));
            }
            Instruction::I32Load16U { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_16U,
                )));
            }
            Instruction::I32Load16S { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_16S,
                )));
            }
            Instruction::I64Load { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::I64,
                )));
            }
            Instruction::F32Load { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::F32,
                )));
            }
            Instruction::F64Load { offset } => {
                results.push(Operand::Local(self.emit_load(
                    &operands[0],
                    *offset as u32,
                    LoadType::F64,
                )));
            }
            Instruction::I32Store { offset } => {
                self.emit_store(&operands[0], *offset as u32, &operands[1], StoreType::I32);
            }
            Instruction::I32Store8 { offset } => {
                self.emit_store(&operands[0], *offset as u32, &operands[1], StoreType::I32_8);
            }
            Instruction::I32Store16 { offset } => {
                self.emit_store(
                    &operands[0],
                    *offset as u32,
                    &operands[1],
                    StoreType::I32_16,
                );
            }
            Instruction::I64Store { offset } => {
                self.emit_store(&operands[0], *offset as u32, &operands[1], StoreType::I64);
            }
            Instruction::F32Store { offset } => {
                self.emit_store(&operands[0], *offset as u32, &operands[1], StoreType::F32);
            }
            Instruction::F64Store { offset } => {
                self.emit_store(&operands[0], *offset as u32, &operands[1], StoreType::F64);
            }
            // As we're going to and from the same ABI, perform no conversions for now
            Instruction::I32FromChar
            | Instruction::I64FromU64
            | Instruction::I64FromS64
            | Instruction::I32FromU32
            | Instruction::I32FromS32
            | Instruction::I32FromU16
            | Instruction::I32FromS16
            | Instruction::I32FromU8
            | Instruction::I32FromS8
            | Instruction::I32FromUsize
            | Instruction::I32FromChar8
            | Instruction::F32FromIf32
            | Instruction::F64FromIf64
            | Instruction::S8FromI32
            | Instruction::U8FromI32
            | Instruction::S16FromI32
            | Instruction::U16FromI32
            | Instruction::S32FromI32
            | Instruction::U32FromI32
            | Instruction::S64FromI64
            | Instruction::U64FromI64
            | Instruction::CharFromI32
            | Instruction::If32FromF32
            | Instruction::If64FromF64
            | Instruction::Char8FromI32
            | Instruction::UsizeFromI32 => {
                results.push(match &operands[0] {
                    Operand::Local(i) => Operand::Local(*i),
                    _ => panic!("expected a local"),
                });
            }
            Instruction::I32FromBorrowedHandle { .. } => unimplemented!(),
            Instruction::I32FromOwnedHandle { .. } => unimplemented!(),
            Instruction::HandleOwnedFromI32 { .. } => unimplemented!(),
            Instruction::HandleBorrowedFromI32 { .. } => unimplemented!(),
            Instruction::ListCanonLower { element, malloc } => {
                // Lifting goes from parent module to adapted module
                assert_eq!(*malloc, Some("witx_malloc"));

                let size = sizeof(element.type_());
                let alignment = alignment(element.type_());

                let (operand, operand_len) = operands[0].split_list();

                let ptr = self.alloc_local(WasmType::I32);

                operand_len.load(&mut self.instructions);
                if size > 1 {
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(size as i32));
                    self.instructions.push(wasm_encoder::Instruction::I32Mul);
                }
                self.instructions
                    .push(wasm_encoder::Instruction::I32Const(alignment as i32));
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.malloc_index));
                // TODO: trap on malloc failure
                self.instructions
                    .push(wasm_encoder::Instruction::LocalTee(ptr));
                operand.load(&mut self.instructions);
                operand_len.load(&mut self.instructions);
                self.instructions
                    .push(wasm_encoder::Instruction::MemoryCopy {
                        src: PARENT_MEMORY_INDEX,
                        dst: ADAPTED_MEMORY_INDEX,
                    });

                results.push(Operand::Pointer {
                    addr: ptr,
                    memory: ADAPTED_MEMORY_INDEX,
                });
                results.push(operand_len);
            }
            Instruction::ListLower { .. } => unreachable!(),
            Instruction::ListCanonLift { element, free } => {
                // Lifting goes from adapted module to parent module
                assert_eq!(*free, Some("witx_free"));

                let size = sizeof(element.type_());
                let alignment = alignment(element.type_());

                let ptr = self.alloc_local(WasmType::I32);

                operands[1].load(&mut self.instructions);
                if size > 1 {
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(size as i32));
                    self.instructions.push(wasm_encoder::Instruction::I32Mul);
                }

                self.instructions
                    .push(wasm_encoder::Instruction::I32Const(alignment as i32));
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.parent_malloc_index));
                // TODO: trap on malloc failure
                self.instructions
                    .push(wasm_encoder::Instruction::LocalTee(ptr));
                operands[0].load(&mut self.instructions);
                operands[1].load(&mut self.instructions);
                self.instructions
                    .push(wasm_encoder::Instruction::MemoryCopy {
                        src: ADAPTED_MEMORY_INDEX,
                        dst: PARENT_MEMORY_INDEX,
                    });

                results.push(Operand::List {
                    addr: ptr,
                    len: operands[1]
                        .local()
                        .expect("expected a local for the length"),
                });
            }
            Instruction::ListLift { .. } => unreachable!(),
            Instruction::IterElem => unimplemented!(),
            Instruction::IterBasePointer => unimplemented!(),
            Instruction::BufferLowerPtrLen { .. } => unimplemented!(),
            Instruction::BufferLowerHandle { .. } => unimplemented!(),
            Instruction::BufferLiftPtrLen { .. } => unimplemented!(),
            Instruction::BufferLiftHandle { .. } => unimplemented!(),
            Instruction::RecordLower { .. } => unimplemented!(),
            Instruction::RecordLift { .. } => unimplemented!(),
            Instruction::I32FromBitflags { .. } => unimplemented!(),
            Instruction::I64FromBitflags { .. } => unimplemented!(),
            Instruction::BitflagsFromI32 { .. } => unimplemented!(),
            Instruction::BitflagsFromI64 { .. } => unimplemented!(),
            Instruction::VariantPayload => unimplemented!(),
            Instruction::VariantLower { .. } => unimplemented!(),
            Instruction::VariantLift { .. } => unimplemented!(),
            Instruction::CallWasm {
                module: _,
                name: _,
                params,
                results: returns,
            } => {
                assert_eq!(operands.len(), params.len());
                for operand in operands.iter() {
                    operand.load(&mut self.instructions);
                }
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.func_index));

                for ty in returns.iter() {
                    let local = self.alloc_local(*ty);
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalSet(local));
                    results.push(Operand::Local(local));
                }
            }
            Instruction::CallInterface { .. } => unimplemented!(),
            Instruction::Return { amt } => {
                // Clean up any retptr allocation in the adapted module
                if let Some((ptr, len)) = &self.retptr_alloc {
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(*ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const((len * 8) as i32));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(8));
                    self.instructions
                        .push(wasm_encoder::Instruction::Call(self.free_index));
                }

                assert!(operands.len() == *amt);

                if let Some(retptr) = self.signature.retptr.clone() {
                    let retptr_index = self.signature.params.len() as u32 - 1;
                    let mut offset = 0;
                    let mut types = retptr.iter();

                    for operand in operands {
                        match operand {
                            Operand::Local(i) => {
                                let ty = types.next().expect("incorrect number of types");
                                assert!(*ty == self.local_type(*i));

                                self.emit_store(
                                    &Operand::Pointer {
                                        addr: retptr_index,
                                        memory: PARENT_MEMORY_INDEX,
                                    },
                                    offset,
                                    operand,
                                    ty.into(),
                                );

                                offset += 8;
                            }
                            Operand::List { addr, len } => {
                                let ty = types.next().expect("incorrect number of types");
                                assert!(*ty == self.local_type(*addr));

                                self.emit_store(
                                    &Operand::Pointer {
                                        addr: retptr_index,
                                        memory: PARENT_MEMORY_INDEX,
                                    },
                                    offset,
                                    &Operand::Local(*addr),
                                    ty.into(),
                                );

                                offset += 8;

                                let ty = types.next().expect("incorrect number of types");
                                assert!(*ty == self.local_type(*addr));

                                self.emit_store(
                                    &Operand::Pointer {
                                        addr: retptr_index,
                                        memory: PARENT_MEMORY_INDEX,
                                    },
                                    offset,
                                    &Operand::Local(*len),
                                    ty.into(),
                                );

                                offset += 8;
                            }
                            _ => panic!("expected a local or list"),
                        }
                    }
                    assert!(types.next().is_none());
                } else {
                    for operand in operands {
                        operand.load(&mut self.instructions);
                    }
                }

                self.instructions.push(wasm_encoder::Instruction::End);
            }
            Instruction::Witx { .. } => unreachable!(),
        }
    }

    fn allocate_typed_space(&mut self, _ty: &NamedType) -> Self::Operand {
        unimplemented!()
    }

    fn allocate_i64_array(&mut self, amt: usize) -> Self::Operand {
        assert!(self.signature.retptr.is_some());
        use wasm_encoder::Instruction;
        // TODO: use shadow stack space from the original module rather than alloc?
        // this would require changing the original module to export the stack global

        let ptr = self.alloc_local(WasmType::I32);
        self.instructions
            .push(Instruction::I32Const(amt as i32 * 8));
        self.instructions.push(Instruction::I32Const(8));
        self.instructions.push(Instruction::Call(self.malloc_index));
        self.instructions.push(Instruction::LocalSet(ptr));

        self.retptr_alloc = Some((ptr, amt as u32));

        Operand::Pointer {
            addr: ptr,
            memory: ADAPTED_MEMORY_INDEX,
        }
    }

    fn push_block(&mut self) {
        unimplemented!()
    }

    fn finish_block(&mut self, _operand: &mut Vec<Self::Operand>) {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::{to_val_type, CodeGenerator};
    use anyhow::Result;
    use wasm_encoder::{
        CodeSection, EntityType, FunctionSection, ImportSection, Limits, MemoryType, Module,
        TypeSection,
    };

    fn generate_adapter(interface: &str) -> Result<String> {
        let module = witx::parse(interface)?;

        let func = module.func(&"test".into()).unwrap();

        let mut generator =
            CodeGenerator::new(module.func(&"test".into()).unwrap().as_ref(), 0, 1, 2, 3);

        func.call(
            &"test".into(),
            witx::CallMode::DeclaredExport,
            &mut generator,
        );

        let params = generator
            .signature
            .params
            .iter()
            .map(to_val_type)
            .collect::<Vec<_>>();
        let results = generator
            .signature
            .results
            .iter()
            .map(to_val_type)
            .collect::<Vec<_>>();
        let func = generator.into_function();

        let mut module = Module::new();

        let mut s = TypeSection::new();
        s.function(params, results);
        // witx_malloc's type
        s.function(
            crate::adapter::MALLOC_FUNC_TYPE
                .params
                .iter()
                .map(crate::adapter::to_val_type),
            crate::adapter::MALLOC_FUNC_TYPE
                .returns
                .iter()
                .map(crate::adapter::to_val_type),
        );
        // witx_free's type
        s.function(
            crate::adapter::FREE_FUNC_TYPE
                .params
                .iter()
                .map(crate::adapter::to_val_type),
            crate::adapter::FREE_FUNC_TYPE
                .returns
                .iter()
                .map(crate::adapter::to_val_type),
        );
        module.section(&s);

        let mut s = ImportSection::new();
        s.import("inner", Some("test"), EntityType::Function(0));
        s.import("$parent", Some("witx_malloc"), EntityType::Function(1));
        s.import("inner", Some("witx_malloc"), EntityType::Function(1));
        s.import("inner", Some("witx_free"), EntityType::Function(2));
        s.import(
            "$parent",
            Some("memory"),
            EntityType::Memory(MemoryType {
                limits: Limits { min: 0, max: None },
            }),
        );
        s.import(
            "inner",
            Some("memory"),
            EntityType::Memory(MemoryType {
                limits: Limits { min: 0, max: None },
            }),
        );
        module.section(&s);

        let mut s = FunctionSection::new();
        s.function(0);
        module.section(&s);

        let mut s = CodeSection::new();
        s.function(&func);
        module.section(&s);

        let bytes = module.finish();

        let mut validator = wasmparser::Validator::new();
        let mut features = wasmparser::WasmFeatures::default();
        features.multi_memory = true;
        validator.wasm_features(features);
        validator.validate_all(&bytes)?;

        wasmprinter::print_bytes(bytes)
    }

    fn expected_output(ty: &str, func: &str) -> String {
        format!(
            "\
(module
  (type (;0;) {})
  (type (;1;) (func (param i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32 i32)))
  (import \"inner\" \"test\" (func (;0;) (type 0)))
  (import \"$parent\" \"witx_malloc\" (func (;1;) (type 1)))
  (import \"inner\" \"witx_malloc\" (func (;2;) (type 1)))
  (import \"inner\" \"witx_free\" (func (;3;) (type 2)))
  (import \"$parent\" \"memory\" (memory (;0;) 0))
  (import \"inner\" \"memory\" (memory (;1;) 0))
  {})",
            ty, func
        )
    }

    #[test]
    fn generates_with_no_parameters() -> Result<()> {
        assert_eq!(
            generate_adapter(r#"(module (export "test" (func)))"#)?,
            expected_output("(func)", "(func (;4;) (type 0)\n    call 0)")
        );

        Ok(())
    }

    #[test]
    fn generates_with_unsigned_integer_params() -> Result<()> {
        assert_eq!(
            generate_adapter(
                r#"(module (export "test" (func (param $p0 u8) (param $p1 u16) (param $p2 u32) (param $p3 u64))))"#
            )?,
            expected_output(
                "(func (param i32 i32 i32 i64))",
                "\
(func (;4;) (type 0) (param i32 i32 i32 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    call 0)"
            )
        );

        Ok(())
    }

    #[test]
    fn generates_with_signed_integer_params() -> Result<()> {
        assert_eq!(
            generate_adapter(
                r#"(module (export "test" (func (param $p0 s8) (param $p1 s16) (param $p2 s32) (param $p3 s64))))"#
            )?,
            expected_output(
                "(func (param i32 i32 i32 i64))",
                "\
(func (;4;) (type 0) (param i32 i32 i32 i64)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    call 0)"
            )
        );

        Ok(())
    }

    #[test]
    fn generates_with_float_params() -> Result<()> {
        assert_eq!(
            generate_adapter(r#"(module (export "test" (func (param $p0 f32) (param $p1 f64))))"#)?,
            expected_output(
                "(func (param f32 f64))",
                "\
(func (;4;) (type 0) (param f32 f64)
    local.get 0
    local.get 1
    call 0)"
            )
        );

        Ok(())
    }

    #[test]
    fn generates_with_integer_result() -> Result<()> {
        for ty in &["u8", "s8", "u16", "s16", "u32", "s32"] {
            assert_eq!(
                generate_adapter(&format!(
                    r#"(module (export "test" (func (result $p0 {}))))"#,
                    ty
                ))?,
                expected_output(
                    "(func (result i32))",
                    "\
(func (;4;) (type 0) (result i32)
    (local i32)
    call 0
    local.set 0
    local.get 0)"
                )
            );
        }

        Ok(())
    }

    #[test]
    fn generates_with_float_result() -> Result<()> {
        assert_eq!(
            generate_adapter(r#"(module (export "test" (func (result $p0 f32))))"#)?,
            expected_output(
                "(func (result f32))",
                "\
(func (;4;) (type 0) (result f32)
    (local f32)
    call 0
    local.set 0
    local.get 0)"
            )
        );

        Ok(())
    }

    #[test]
    fn generates_with_strings() -> Result<()> {
        assert_eq!(
            generate_adapter(
                r#"(module (export "test" (func (param $p0 u32) (param $p1 string) (result $r0 string))))"#
            )?,
            expected_output(
                "(func (param i32 i32 i32 i32))",
                "\
(func (;4;) (type 0) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    local.get 2
    i32.const 1
    call 2
    local.tee 4
    local.get 1
    local.get 2
    memory.copy 1 0
    i32.const 16
    i32.const 8
    call 2
    local.set 5
    local.get 0
    local.get 4
    local.get 2
    local.get 5
    call 0
    local.get 5
    i32.load (memory 1)
    local.set 6
    local.get 5
    i32.load (memory 1) offset=8
    local.set 7
    local.get 7
    i32.const 1
    call 1
    local.tee 8
    local.get 6
    local.get 7
    memory.copy 0 1
    local.get 5
    i32.const 16
    i32.const 8
    call 3
    local.get 3
    local.get 8
    i32.store
    local.get 3
    local.get 7
    i32.store offset=8)"
            )
        );

        Ok(())
    }

    #[test]
    fn generates_with_multiple_returns() -> Result<()> {
        assert_eq!(
            generate_adapter(
                r#"(module (export "test" (func (param $p0 s32) (param $p1 s16) (result $r0 s8) (result $r1 s64) (result $r2 string) (result $r3 f64) (result $r4 f32))))"#
            )?,
            expected_output(
                "(func (param i32 i32 i32))",
                "\
(func (;4;) (type 0) (param i32 i32 i32)
    (local i32 i32 i64 i32 i32 f64 f32 i32)
    i32.const 48
    i32.const 8
    call 2
    local.set 3
    local.get 0
    local.get 1
    local.get 3
    call 0
    local.get 3
    i32.load (memory 1)
    local.set 4
    local.get 3
    i64.load (memory 1) offset=8 align=4
    local.set 5
    local.get 3
    i32.load (memory 1) offset=16
    local.set 6
    local.get 3
    i32.load (memory 1) offset=24
    local.set 7
    local.get 3
    f64.load (memory 1) offset=32 align=4
    local.set 8
    local.get 3
    f32.load (memory 1) offset=40
    local.set 9
    local.get 7
    i32.const 1
    call 1
    local.tee 10
    local.get 6
    local.get 7
    memory.copy 0 1
    local.get 3
    i32.const 48
    i32.const 8
    call 3
    local.get 2
    local.get 4
    i32.store
    local.get 2
    local.get 5
    i64.store offset=8 align=4
    local.get 2
    local.get 10
    i32.store offset=16
    local.get 2
    local.get 7
    i32.store offset=24
    local.get 2
    local.get 8
    f64.store offset=32 align=4
    local.get 2
    local.get 9
    f32.store offset=40)"
            )
        );

        Ok(())
    }
}
