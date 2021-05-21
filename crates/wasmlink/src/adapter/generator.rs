use witx::{
    Bindgen, BuiltinType, CallMode, Function, Instruction, NamedType, Type, WasmSignature, WasmType,
};

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

pub struct CodeGenerator<'a> {
    signature: WasmSignature,
    locals: Vec<WasmType>,
    instructions: Vec<wasm_encoder::Instruction<'a>>,
    index: u32,
    caller_malloc: u32,
    malloc: u32,
    free: u32,
    cleanup: Option<(u32, u32)>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(func: &Function, index: u32, caller_malloc: u32, malloc: u32, free: u32) -> Self {
        let signature = func.wasm_signature(CallMode::DeclaredExport);
        let mut locals = Vec::new();
        for param in signature.params.iter() {
            locals.push(*param);
        }
        Self {
            signature,
            locals,
            instructions: Vec::new(),
            index,
            caller_malloc,
            malloc,
            free,
            cleanup: None,
        }
    }

    pub fn into_function(self) -> wasm_encoder::Function {
        let mut function = wasm_encoder::Function::new(
            self.locals
                .iter()
                .skip(self.signature.params.len())
                .map(|ty| (1, to_val_type(ty))),
        );

        for inst in self.instructions {
            function.instruction(inst);
        }

        function
    }

    fn alloc_local(&mut self, ty: WasmType) -> u32 {
        let index = self.locals.len();
        self.locals.push(ty);
        index as u32
    }
}

#[derive(Debug, Clone)]
pub enum Operand {
    Local(u32),
    LocalPtr { ptr: u32, memory: u32 },
    LocalPtrLen { ptr: u32, len: u32, memory: u32 },
    I32Const(i32),
}

impl Operand {
    fn as_inst<'a>(&self) -> wasm_encoder::Instruction<'a> {
        match self {
            Self::Local(ptr) | Self::LocalPtr { ptr, .. } | Self::LocalPtrLen { ptr, .. } => {
                wasm_encoder::Instruction::LocalGet(*ptr)
            }
            Self::I32Const(i) => wasm_encoder::Instruction::I32Const(*i),
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
        use wasm_encoder::MemArg;

        match inst {
            Instruction::GetArg { nth } => {
                assert!(*nth < self.signature.params.len());
                results.push(Operand::Local(*nth as u32));
            }
            Instruction::I32Const { val } => {
                results.push(Operand::I32Const(*val));
            }
            Instruction::Bitcasts { .. } => unimplemented!(),
            Instruction::ConstZero { .. } => unimplemented!(),
            Instruction::I32Load { offset } => match operands[0] {
                Operand::LocalPtr { ptr, memory } => {
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Load(MemArg {
                            offset: *offset as u32,
                            align: 2,
                            memory_index: memory,
                        }));

                    let local = self.alloc_local(WasmType::I32);
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalSet(local));

                    results.push(Operand::Local(local));
                }
                _ => panic!("load must be via a local"),
            },
            Instruction::I32Load8U { .. } => unimplemented!(),
            Instruction::I32Load8S { .. } => unimplemented!(),
            Instruction::I32Load16U { .. } => unimplemented!(),
            Instruction::I32Load16S { .. } => unimplemented!(),
            Instruction::I64Load { .. } => unimplemented!(),
            Instruction::F32Load { .. } => unimplemented!(),
            Instruction::F64Load { .. } => unimplemented!(),
            Instruction::I32Store { .. } => unimplemented!(),
            Instruction::I32Store8 { .. } => unimplemented!(),
            Instruction::I32Store16 { .. } => unimplemented!(),
            Instruction::I64Store { .. } => unimplemented!(),
            Instruction::F32Store { .. } => unimplemented!(),
            Instruction::F64Store { .. } => unimplemented!(),
            Instruction::I32FromChar => unimplemented!(),
            Instruction::I64FromU64 => unimplemented!(),
            Instruction::I64FromS64 => unimplemented!(),
            Instruction::I32FromU32 => unimplemented!(),
            Instruction::I32FromS32 => unimplemented!(),
            Instruction::I32FromU16 => unimplemented!(),
            Instruction::I32FromS16 => unimplemented!(),
            Instruction::I32FromU8 => unimplemented!(),
            Instruction::I32FromS8 => unimplemented!(),
            Instruction::I32FromUsize => unimplemented!(),
            Instruction::I32FromChar8 => unimplemented!(),
            Instruction::F32FromIf32 => unimplemented!(),
            Instruction::F64FromIf64 => unimplemented!(),
            Instruction::S8FromI32 => unimplemented!(),
            Instruction::U8FromI32 => unimplemented!(),
            Instruction::S16FromI32 => unimplemented!(),
            Instruction::U16FromI32 => unimplemented!(),
            Instruction::S32FromI32 => unimplemented!(),
            Instruction::U32FromI32 => unimplemented!(),
            Instruction::S64FromI64 => unimplemented!(),
            Instruction::U64FromI64 => unimplemented!(),
            Instruction::CharFromI32 => unimplemented!(),
            Instruction::If32FromF32 => unimplemented!(),
            Instruction::If64FromF64 => unimplemented!(),
            Instruction::Char8FromI32 => unimplemented!(),
            Instruction::UsizeFromI32 => unimplemented!(),
            Instruction::I32FromBorrowedHandle { .. } => unimplemented!(),
            Instruction::I32FromOwnedHandle { .. } => unimplemented!(),
            Instruction::HandleOwnedFromI32 { .. } => unimplemented!(),
            Instruction::HandleBorrowedFromI32 { .. } => unimplemented!(),
            Instruction::ListCanonLower { element, malloc } => {
                assert_eq!(*malloc, Some("witx_malloc"));

                // Lowering goes from caller to adapted module
                // input is local to get the list from
                // assumption: local with index "input + 1" stores the length of the list
                // Return values are:
                // * address in memory 1 containing list
                // * length of list
                let size = sizeof(element.type_());
                let alignment = alignment(element.type_());

                let mut operand_len = match &operands[0] {
                    Operand::Local(i) => Operand::Local(i + 1),
                    _ => panic!("list lowering operand must be a local"),
                };

                // Allocate a local for the result and length
                let ptr = self.alloc_local(WasmType::I32);

                self.instructions.push(operand_len.as_inst());
                if size > 1 {
                    let len = self.alloc_local(WasmType::I32);

                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(size as i32));
                    self.instructions.push(wasm_encoder::Instruction::I32Mul);
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalTee(len));

                    operand_len = Operand::Local(len);
                }
                self.instructions
                    .push(wasm_encoder::Instruction::I32Const(alignment as i32));
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.malloc));
                // TODO: trap on malloc failure
                self.instructions
                    .push(wasm_encoder::Instruction::LocalTee(ptr));
                self.instructions.push(operands[0].as_inst());
                self.instructions.push(operand_len.as_inst());
                self.instructions
                    .push(wasm_encoder::Instruction::MemoryCopy { src: 0, dst: 1 });

                results.push(Operand::LocalPtr { ptr, memory: 1 });
                results.push(operand_len);
            }
            Instruction::ListLower { .. } => unimplemented!(),
            Instruction::ListCanonLift { element, free } => {
                // Lifting goes from adapted module to caller
                assert_eq!(*free, Some("witx_free"));

                let size = sizeof(element.type_());
                let alignment = alignment(element.type_());

                // Allocate a local for the result and length
                let ptr = self.alloc_local(WasmType::I32);

                self.instructions.push(operands[1].as_inst());
                if size > 1 {
                    let len = self.alloc_local(WasmType::I32);

                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(size as i32));
                    self.instructions.push(wasm_encoder::Instruction::I32Mul);
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalTee(len));

                    operands[1] = Operand::Local(len);
                }

                self.instructions
                    .push(wasm_encoder::Instruction::I32Const(alignment as i32));
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.caller_malloc));
                // TODO: trap on malloc failure
                self.instructions
                    .push(wasm_encoder::Instruction::LocalTee(ptr));
                self.instructions.push(operands[0].as_inst());
                self.instructions.push(operands[1].as_inst());
                self.instructions
                    .push(wasm_encoder::Instruction::MemoryCopy { src: 1, dst: 0 });

                results.push(Operand::LocalPtrLen {
                    ptr,
                    memory: 0,
                    len: match &operands[1] {
                        Operand::Local(i) => *i,
                        _ => panic!("length must be stored in a local"),
                    },
                });
            }
            Instruction::ListLift { .. } => unimplemented!(),
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
                results,
            } => {
                assert_eq!(operands.len(), params.len());
                for operand in operands.iter() {
                    self.instructions.push(operand.as_inst());
                }
                self.instructions
                    .push(wasm_encoder::Instruction::Call(self.index));

                assert!(results.is_empty()); // TODO: support return values
            }
            Instruction::CallInterface { .. } => unimplemented!(),
            Instruction::Return { .. } => {
                if let Some((ptr, len)) = &self.cleanup {
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(*ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const((len * 8) as i32));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Const(8));
                    self.instructions
                        .push(wasm_encoder::Instruction::Call(self.free));
                }

                if let Some(retptr) = &self.signature.retptr {
                    if retptr != &[WasmType::I32, WasmType::I32] {
                        // TODO: support other retptr forms
                        unimplemented!()
                    }

                    assert!(!self.signature.params.is_empty());
                    let out_ptr = self.signature.params.len() as u32 - 1;

                    let (ptr, len, memory) = match operands[0] {
                        Operand::LocalPtrLen { ptr, len, memory } => (ptr, len, memory),
                        _ => panic!("expected ptr-len pair"),
                    };

                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(out_ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: memory,
                        }));
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(out_ptr));
                    self.instructions
                        .push(wasm_encoder::Instruction::LocalGet(len));
                    self.instructions
                        .push(wasm_encoder::Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: memory,
                        }));
                    self.instructions.push(wasm_encoder::Instruction::End);
                } else {
                    // TODO: support other return values
                    unimplemented!()
                }
            }
            Instruction::Witx { .. } => unimplemented!(),
        }
    }

    fn allocate_typed_space(&mut self, _ty: &NamedType) -> Self::Operand {
        unimplemented!()
    }

    fn allocate_i64_array(&mut self, amt: usize) -> Self::Operand {
        use wasm_encoder::Instruction;
        // TODO: use shadow stack space from the original module rather than alloc?
        // this would require changing the original module to export the stack global

        let ptr = self.alloc_local(WasmType::I32);
        self.instructions
            .push(Instruction::I32Const(amt as i32 * 8));
        self.instructions.push(Instruction::I32Const(8));
        self.instructions.push(Instruction::Call(self.malloc));
        self.instructions.push(Instruction::LocalSet(ptr));

        self.cleanup = Some((ptr, amt as u32));

        Operand::LocalPtr { ptr, memory: 1 }
    }

    fn push_block(&mut self) {
        unimplemented!()
    }

    fn finish_block(&mut self, _operand: &mut Vec<Self::Operand>) {
        unimplemented!()
    }
}
