use crate::adapted::FREE_EXPORT_NAME;
use itertools::Either;
use witx2::{
    abi::{Bindgen, CallMode, Instruction, WasmSignature, WasmType},
    Function, Interface, Record, RecordKind, SizeAlign, Type, TypeDefKind, TypeId,
};

// The parent's memory is imported, so it is always index 0 for the adapter logic
const PARENT_MEMORY_INDEX: u32 = 0;
// The adapted module's memory is aliased, so it is always index 1 for the adapter logic
const ADAPTED_MEMORY_INDEX: u32 = 1;

// Represents a placeholder "global" that will be translated to the local storing the
// base pointer to the list being written to when lowering/lifting non-canonical lists
const BASE_POINTER_GLOBAL: u32 = 0;
// Represents a placeholder "global" that will be translated to a load instruction
// from the list being read from when lowering/lifting non-canonical lists
// This value is the *base* for offsets within the element being lifted/lowered.
// For example, a reference to global `1` means offset 0 from the start of the element
// and global `5` means offset 4 from the start of the element.
const ELEMENT_GLOBAL: u32 = 1;

fn to_val_type(ty: &WasmType) -> wasm_encoder::ValType {
    match ty {
        WasmType::I32 => wasm_encoder::ValType::I32,
        WasmType::I64 => wasm_encoder::ValType::I64,
        WasmType::F32 => wasm_encoder::ValType::F32,
        WasmType::F64 => wasm_encoder::ValType::F64,
    }
}

fn param_to_operand(interface: &Interface, index: u32, ty: &Type) -> (u32, Operand) {
    match ty {
        Type::Id(id) => match &interface.types.get(*id).unwrap().kind {
            TypeDefKind::Record(r) => {
                let (count, fields) = match r.kind {
                    RecordKind::Flags(_) => match interface.flags_repr(r) {
                        Some(_) => (1, vec![Box::new(Operand::Local(index))]),
                        None => {
                            let mut fields = Vec::with_capacity(r.num_i32s());
                            for i in 0..fields.len() as u32 {
                                fields.push(Box::new(Operand::Local(index + i)));
                            }
                            (fields.len() as u32, fields)
                        }
                    },
                    RecordKind::Tuple | RecordKind::Other => {
                        let mut fields = Vec::new();
                        let mut count = 0;
                        for f in &r.fields {
                            let (local, operand) =
                                param_to_operand(interface, index + count, &f.ty);
                            fields.push(Box::new(operand));
                            count += local;
                        }
                        (count, fields)
                    }
                };
                (count, Operand::Record { fields })
            }
            TypeDefKind::Variant(_) => todo!(),
            TypeDefKind::List(_) => (
                2,
                Operand::List {
                    addr: index,
                    len: index + 1,
                },
            ),
            TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => (1, Operand::Local(index)),
            TypeDefKind::PushBuffer(_) => todo!(),
            TypeDefKind::PullBuffer(_) => todo!(),
            TypeDefKind::Type(t) => param_to_operand(interface, index, t),
        },
        _ => (1, Operand::Local(index)),
    }
}

fn offsets_for_type(interface: &Interface, sizes: &SizeAlign, ty: &Type) -> Vec<(u32, WasmType)> {
    fn _offsets_for_type(
        interface: &Interface,
        sizes: &SizeAlign,
        ty: &Type,
        current: u32,
        offsets: &mut Vec<(u32, WasmType)>,
    ) {
        match ty {
            Type::Id(id) => match &interface.types[*id].kind {
                TypeDefKind::Type(t) => _offsets_for_type(interface, sizes, t, current, offsets),
                TypeDefKind::List(_) => {
                    offsets.push((current, WasmType::I32));
                    offsets.push((current + 4, WasmType::I32));
                }
                TypeDefKind::Pointer(_) | TypeDefKind::ConstPointer(_) => {
                    offsets.push((current, WasmType::I32));
                }
                TypeDefKind::PushBuffer(_) | TypeDefKind::PullBuffer(_) => todo!(),
                TypeDefKind::Record(r) => {
                    for (f, o) in r.fields.iter().zip(sizes.field_offsets(r).iter()) {
                        _offsets_for_type(interface, sizes, &f.ty, current + *o as u32, offsets);
                    }
                }
                TypeDefKind::Variant(_) => todo!(),
            },
            Type::S8
            | Type::U8
            | Type::S16
            | Type::U16
            | Type::S32
            | Type::U32
            | Type::Char
            | Type::Handle(_)
            | Type::CChar
            | Type::Usize => offsets.push((current, WasmType::I32)),
            Type::U64 | Type::S64 => offsets.push((current, WasmType::I64)),
            Type::F32 => offsets.push((current, WasmType::F32)),
            Type::F64 => offsets.push((current, WasmType::F64)),
        }
    }

    let mut offsets = Vec::new();
    _offsets_for_type(interface, sizes, ty, 0, &mut offsets);
    offsets
}

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Clone)]
pub enum Operand {
    Local(u32),
    I32Const(i32),
    Pointer { addr: u32, memory: u32 },
    List { addr: u32, len: u32 },
    Record { fields: Vec<Box<Operand>> },
    Global(u32),
    GlobalPointer { addr: u32, memory: u32 },
}

impl Operand {
    fn local(&self) -> Option<u32> {
        match self {
            Operand::Local(i) => Some(*i),
            _ => None,
        }
    }

    fn locals(&self) -> Vec<u32> {
        fn _locals(o: &Operand, v: &mut Vec<u32>) {
            match o {
                Operand::Local(i) | Operand::Pointer { addr: i, .. } => v.push(*i),
                Operand::List { addr, len } => {
                    v.push(*addr);
                    v.push(*len);
                }
                Operand::Record { fields } => {
                    for f in fields {
                        _locals(f, v);
                    }
                }
                _ => {}
            }
        }

        let mut v = Vec::new();
        _locals(self, &mut v);
        v
    }

    fn load(&self, generator: &mut CodeGenerator) {
        match self {
            Self::Local(i) | Self::Pointer { addr: i, .. } => {
                generator.emit_instruction(wasm_encoder::Instruction::LocalGet(*i));
            }
            Self::I32Const(i) => {
                generator.emit_instruction(wasm_encoder::Instruction::I32Const(*i));
            }
            Self::List { .. } => panic!("list operands must be lowered"),
            Self::Record { fields } => {
                for field in fields {
                    field.load(generator);
                }
            }
            Self::Global(i) | Self::GlobalPointer { addr: i, .. } => {
                generator.emit_instruction(wasm_encoder::Instruction::GlobalGet(*i));
            }
        }
    }

    fn lower_list(self) -> (Self, Self) {
        match self {
            Self::List { addr, len } => (Self::Local(addr), Self::Local(len)),
            Self::Global(i) => (Self::Global(i), Self::Global(i + 4 /* offset */)),
            _ => panic!("expected a list operand to lower, found: {:?}", self),
        }
    }

    fn lower_record<'a>(
        self,
        sizes: &SizeAlign,
        record: &'a Record,
    ) -> impl Iterator<Item = Operand> + 'a {
        match self {
            Self::Record { fields } => Either::Left(fields.into_iter().map(|f| *f)),
            Self::Global(i) => Either::Right(
                sizes
                    .field_offsets(record)
                    .into_iter()
                    .map(move |o| Operand::Global(i + o as u32)),
            ),
            _ => panic!("expected a record operand to lower, found: {:?}", self),
        }
    }
}

pub struct CodeGenerator<'a> {
    params: Vec<Operand>,
    signature: &'a WasmSignature,
    locals: Vec<WasmType>,
    instructions: Vec<Vec<wasm_encoder::Instruction<'a>>>,
    blocks: Vec<(Vec<Operand>, Vec<wasm_encoder::Instruction<'a>>)>,
    locals_start_index: u32,
    func_index: u32,
    parent_realloc_index: u32,
    realloc_index: u32,
    sizes: SizeAlign,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(
        interface: &Interface,
        func: &Function,
        signature: &'a WasmSignature,
        func_index: u32,
        parent_realloc_index: u32,
        realloc_index: u32,
    ) -> Self {
        let mut locals_start_index = 0;
        let params = func
            .params
            .iter()
            .map(|(_, ty)| {
                let (count, operand) = param_to_operand(interface, locals_start_index, ty);
                locals_start_index += count;
                operand
            })
            .collect();

        // Account for any return pointer in the parameters
        if signature.retptr.is_some() {
            locals_start_index += 1;
        }

        let mut sizes = SizeAlign::default();
        sizes.fill(CallMode::WasmExport, interface);

        Self {
            params,
            signature,
            locals: Vec::new(),
            instructions: Vec::new(),
            blocks: Vec::new(),
            locals_start_index,
            func_index,
            parent_realloc_index,
            realloc_index,
            sizes,
        }
    }

    pub fn into_function(mut self) -> wasm_encoder::Function {
        let mut function =
            wasm_encoder::Function::new(self.locals.iter().map(|ty| (1, to_val_type(ty))));

        assert!(self.instructions.len() == 1);

        for inst in self.instructions.swap_remove(0) {
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

    fn emit_instruction(&mut self, inst: wasm_encoder::Instruction<'a>) {
        if self.instructions.is_empty() {
            self.instructions.push(Vec::new());
        }
        self.instructions.last_mut().unwrap().push(inst);
    }

    fn emit_load(&mut self, offset: u32, ty: LoadType, memory_index: u32) -> WasmType {
        let memarg = wasm_encoder::MemArg {
            offset: offset as u32,
            align: match ty {
                LoadType::I32 => 2,
                LoadType::I32_8S | LoadType::I32_8U => 0,
                LoadType::I32_16S | LoadType::I32_16U => 1,
                LoadType::I64 => 3,
                LoadType::F32 => 2,
                LoadType::F64 => 3,
            },
            memory_index,
        };

        let (wasm_ty, inst) = match ty {
            LoadType::I32 => (WasmType::I32, wasm_encoder::Instruction::I32Load(memarg)),
            LoadType::I32_8U => (WasmType::I32, wasm_encoder::Instruction::I32Load8_U(memarg)),
            LoadType::I32_8S => (WasmType::I32, wasm_encoder::Instruction::I32Load8_S(memarg)),
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

        self.emit_instruction(inst);
        wasm_ty
    }

    fn emit_load_to_local(&mut self, addr: &Operand, offset: u32, ty: LoadType) -> u32 {
        let (addr, memory) = match addr {
            Operand::Pointer { addr, memory } => (Operand::Local(*addr), memory),
            Operand::GlobalPointer { addr, memory } => (Operand::Global(*addr), memory),
            _ => panic!("expected pointer for load, found: {:?}", addr),
        };

        addr.load(self);

        let ty = self.emit_load(offset, ty, *memory);
        let local = self.alloc_local(ty);
        self.emit_instruction(wasm_encoder::Instruction::LocalSet(local));
        local
    }

    fn emit_store(&mut self, offset: u32, ty: StoreType, memory_index: u32) {
        let memarg = wasm_encoder::MemArg {
            offset: offset as u32,
            align: match ty {
                StoreType::I32 => 2,
                StoreType::I32_8 => 0,
                StoreType::I32_16 => 1,
                StoreType::I64 => 3,
                StoreType::F32 => 2,
                StoreType::F64 => 3,
            },
            memory_index,
        };

        let inst = match ty {
            StoreType::I32 => wasm_encoder::Instruction::I32Store(memarg),
            StoreType::I32_8 => wasm_encoder::Instruction::I32Store8(memarg),
            StoreType::I32_16 => wasm_encoder::Instruction::I32Store16(memarg),
            StoreType::I64 => wasm_encoder::Instruction::I64Store(memarg),
            StoreType::F32 => wasm_encoder::Instruction::F32Store(memarg),
            StoreType::F64 => wasm_encoder::Instruction::F64Store(memarg),
        };

        self.emit_instruction(inst);
    }

    fn emit_store_from_local(
        &mut self,
        addr: &Operand,
        offset: u32,
        local: &Operand,
        ty: StoreType,
    ) {
        addr.load(self);
        local.load(self);

        match addr {
            Operand::Pointer { memory, .. } | Operand::GlobalPointer { memory, .. } => {
                let memarg = wasm_encoder::MemArg {
                    offset: offset as u32,
                    align: match ty {
                        StoreType::I32 => 2,
                        StoreType::I32_8 => 0,
                        StoreType::I32_16 => 1,
                        StoreType::I64 => 3,
                        StoreType::F32 => 2,
                        StoreType::F64 => 3,
                    },
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

                self.emit_instruction(inst);
            }
            _ => panic!("expected a pointer for first operand, found: {:?}", addr),
        }
    }

    fn emit_copy_list(
        &mut self,
        size: u32,
        alignment: u32,
        list: &Operand,
        len: &Operand,
        lowering: bool,
        offsets: Option<Vec<(u32, WasmType)>>,
    ) -> u32 {
        self.emit_instruction(wasm_encoder::Instruction::I32Const(0)); // Previous ptr
        self.emit_instruction(wasm_encoder::Instruction::I32Const(0)); // Previous size

        let ptr = self.alloc_local(WasmType::I32);
        len.load(self);
        if size > 1 {
            self.emit_instruction(wasm_encoder::Instruction::I32Const(size as i32));
            self.emit_instruction(wasm_encoder::Instruction::I32Mul);
        }
        self.emit_instruction(wasm_encoder::Instruction::I32Const(alignment as i32));
        self.emit_instruction(wasm_encoder::Instruction::Call(if lowering {
            self.realloc_index
        } else {
            self.parent_realloc_index
        }));
        // TODO: trap on alloc failure

        // If given element offsets, copy each element individually in a loop
        if let Some(offsets) = offsets {
            self.emit_instruction(wasm_encoder::Instruction::LocalSet(ptr));

            let counter = self.alloc_local(WasmType::I32);
            self.emit_instruction(wasm_encoder::Instruction::I32Const(0));
            self.emit_instruction(wasm_encoder::Instruction::LocalSet(counter));
            let element_offset = self.alloc_local(WasmType::I32);
            self.emit_instruction(wasm_encoder::Instruction::I32Const(0));
            self.emit_instruction(wasm_encoder::Instruction::LocalSet(element_offset));

            self.emit_instruction(wasm_encoder::Instruction::Block(
                wasm_encoder::BlockType::Empty,
            ));
            self.emit_instruction(wasm_encoder::Instruction::Loop(
                wasm_encoder::BlockType::Empty,
            ));

            // Wasm: if counter >= len { break }
            self.emit_instruction(wasm_encoder::Instruction::LocalGet(counter));
            len.load(self);
            self.emit_instruction(wasm_encoder::Instruction::I32Eq);
            self.emit_instruction(wasm_encoder::Instruction::BrIf(1));

            // Add the block to the loop, replacing global references with the source and destination
            assert!(!self.blocks.is_empty());
            let (operands, block) = self.blocks.pop().unwrap();
            for mut inst in block {
                // Fix up any load and store instructions to use the correct memory
                match &mut inst {
                    wasm_encoder::Instruction::I32Store8(arg)
                    | wasm_encoder::Instruction::I32Store16(arg)
                    | wasm_encoder::Instruction::I32Store(arg)
                    | wasm_encoder::Instruction::I64Store8(arg)
                    | wasm_encoder::Instruction::I64Store16(arg)
                    | wasm_encoder::Instruction::I64Store32(arg)
                    | wasm_encoder::Instruction::I64Store(arg)
                    | wasm_encoder::Instruction::F32Store(arg)
                    | wasm_encoder::Instruction::F64Store(arg) => {
                        arg.memory_index = if lowering {
                            ADAPTED_MEMORY_INDEX
                        } else {
                            PARENT_MEMORY_INDEX
                        };
                    }
                    wasm_encoder::Instruction::I32Load8_S(arg)
                    | wasm_encoder::Instruction::I32Load8_U(arg)
                    | wasm_encoder::Instruction::I32Load16_S(arg)
                    | wasm_encoder::Instruction::I32Load16_U(arg)
                    | wasm_encoder::Instruction::I32Load(arg)
                    | wasm_encoder::Instruction::I64Load8_S(arg)
                    | wasm_encoder::Instruction::I64Load8_U(arg)
                    | wasm_encoder::Instruction::I64Load16_S(arg)
                    | wasm_encoder::Instruction::I64Load16_U(arg)
                    | wasm_encoder::Instruction::I64Load32_S(arg)
                    | wasm_encoder::Instruction::I64Load32_U(arg)
                    | wasm_encoder::Instruction::I64Load(arg)
                    | wasm_encoder::Instruction::F32Load(arg)
                    | wasm_encoder::Instruction::F64Load(arg) => {
                        arg.memory_index = if lowering {
                            PARENT_MEMORY_INDEX
                        } else {
                            ADAPTED_MEMORY_INDEX
                        };
                    }
                    _ => {}
                };

                match inst {
                    wasm_encoder::Instruction::GlobalGet(BASE_POINTER_GLOBAL) => {
                        self.emit_instruction(wasm_encoder::Instruction::LocalGet(if lowering {
                            ptr
                        } else {
                            list.local().unwrap()
                        }));
                        self.emit_instruction(wasm_encoder::Instruction::LocalGet(element_offset));
                        self.emit_instruction(wasm_encoder::Instruction::I32Add);
                    }
                    wasm_encoder::Instruction::GlobalGet(n) => {
                        let (offset, ty) = offsets
                            .iter()
                            .find(|(o, _)| *o == n - ELEMENT_GLOBAL)
                            .unwrap();
                        assert!(lowering);
                        list.load(self);
                        self.emit_instruction(wasm_encoder::Instruction::LocalGet(element_offset));
                        self.emit_instruction(wasm_encoder::Instruction::I32Add);
                        self.emit_load(
                            *offset,
                            match *ty {
                                WasmType::I32 => LoadType::I32,
                                WasmType::I64 => LoadType::I64,
                                WasmType::F32 => LoadType::F32,
                                WasmType::F64 => LoadType::F64,
                            },
                            PARENT_MEMORY_INDEX,
                        );
                    }
                    _ => {
                        self.emit_instruction(inst);
                    }
                }
            }

            if lowering {
                // Block operand stack should be empty when lowering
                assert!(operands.is_empty());
            } else {
                assert_eq!(operands.len(), 1);

                for (index, local) in operands[0].locals().into_iter().enumerate() {
                    let (offset, ty) = &offsets[index];
                    self.emit_instruction(wasm_encoder::Instruction::LocalGet(ptr));
                    self.emit_instruction(wasm_encoder::Instruction::LocalGet(element_offset));
                    self.emit_instruction(wasm_encoder::Instruction::I32Add);
                    self.emit_instruction(wasm_encoder::Instruction::LocalGet(local));
                    self.emit_store(
                        *offset,
                        match *ty {
                            WasmType::I32 => StoreType::I32,
                            WasmType::I64 => StoreType::I64,
                            WasmType::F32 => StoreType::F32,
                            WasmType::F64 => StoreType::F64,
                        },
                        PARENT_MEMORY_INDEX,
                    );
                }
            }

            // Wasm: counter += 1
            self.emit_instruction(wasm_encoder::Instruction::LocalGet(counter));
            self.emit_instruction(wasm_encoder::Instruction::I32Const(1));
            self.emit_instruction(wasm_encoder::Instruction::I32Add);
            self.emit_instruction(wasm_encoder::Instruction::LocalSet(counter));

            // Wasm: offset += size
            self.emit_instruction(wasm_encoder::Instruction::LocalGet(element_offset));
            self.emit_instruction(wasm_encoder::Instruction::I32Const(size as i32));
            self.emit_instruction(wasm_encoder::Instruction::I32Add);
            self.emit_instruction(wasm_encoder::Instruction::LocalSet(element_offset));

            // Wasm: goto loop
            self.emit_instruction(wasm_encoder::Instruction::Br(0));
            self.emit_instruction(wasm_encoder::Instruction::End);
            self.emit_instruction(wasm_encoder::Instruction::End);
        } else {
            // No offsets given; do a memcpy
            self.emit_instruction(wasm_encoder::Instruction::LocalTee(ptr));
            list.load(self);
            len.load(self);
            if size > 1 {
                self.emit_instruction(wasm_encoder::Instruction::I32Const(size as i32));
                self.emit_instruction(wasm_encoder::Instruction::I32Mul);
            }
            self.emit_instruction(wasm_encoder::Instruction::MemoryCopy {
                src: if lowering {
                    PARENT_MEMORY_INDEX
                } else {
                    ADAPTED_MEMORY_INDEX
                },
                dst: if lowering {
                    ADAPTED_MEMORY_INDEX
                } else {
                    PARENT_MEMORY_INDEX
                },
            });
        }

        ptr
    }
}

impl<'a> Bindgen for CodeGenerator<'a> {
    type Operand = Operand;

    fn emit(
        &mut self,
        interface: &Interface,
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
            Instruction::Bitcasts { .. } => todo!(),
            Instruction::ConstZero { .. } => todo!(),
            Instruction::I32Load { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32,
                )));
            }
            Instruction::I32Load8U { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_8U,
                )));
            }
            Instruction::I32Load8S { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_8S,
                )));
            }
            Instruction::I32Load16U { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_16U,
                )));
            }
            Instruction::I32Load16S { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I32_16S,
                )));
            }
            Instruction::I64Load { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::I64,
                )));
            }
            Instruction::F32Load { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::F32,
                )));
            }
            Instruction::F64Load { offset } => {
                results.push(Operand::Local(self.emit_load_to_local(
                    &operands[0],
                    *offset as u32,
                    LoadType::F64,
                )));
            }
            Instruction::I32Store { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::I32,
                );
            }
            Instruction::I32Store8 { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::I32_8,
                );
            }
            Instruction::I32Store16 { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::I32_16,
                );
            }
            Instruction::I64Store { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::I64,
                );
            }
            Instruction::F32Store { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::F32,
                );
            }
            Instruction::F64Store { offset } => {
                self.emit_store_from_local(
                    &operands[1],
                    *offset as u32,
                    &operands[0],
                    StoreType::F64,
                );
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
                    Operand::Global(i) => Operand::Global(*i),
                    _ => panic!("expected a local or global, found: {:?}", operands[0]),
                });
            }
            Instruction::I32FromBorrowedHandle { .. } => todo!(),
            Instruction::I32FromOwnedHandle { .. } => todo!(),
            Instruction::HandleOwnedFromI32 { .. } => todo!(),
            Instruction::HandleBorrowedFromI32 { .. } => todo!(),
            Instruction::ListCanonLower { element, realloc } => {
                // Lowering goes from parent module to adapted module
                assert_eq!(*realloc, Some(super::REALLOC_EXPORT_NAME));

                let (size, alignment) = match element {
                    Type::Char => (1, 1), // UTF-8
                    _ => (
                        self.sizes.size(element) as u32,
                        self.sizes.align(element) as u32,
                    ),
                };

                let (list, len) = operands.swap_remove(0).lower_list();
                let ptr = self.emit_copy_list(size, alignment, &list, &len, true, None);

                results.push(Operand::Pointer {
                    addr: ptr,
                    memory: ADAPTED_MEMORY_INDEX,
                });
                results.push(len);
            }
            Instruction::ListLower { element, realloc } => {
                // Lowering goes from parent module to adapted module
                assert_eq!(*realloc, Some(super::REALLOC_EXPORT_NAME));

                let offsets = offsets_for_type(interface, &self.sizes, element);
                let size = self.sizes.size(element) as u32;
                let alignment = self.sizes.align(element) as u32;

                let (list, len) = operands.swap_remove(0).lower_list();
                let ptr = self.emit_copy_list(size, alignment, &list, &len, true, Some(offsets));

                results.push(Operand::Pointer {
                    addr: ptr,
                    memory: ADAPTED_MEMORY_INDEX,
                });
                results.push(len);
            }
            Instruction::ListCanonLift { element, free } => {
                assert_eq!(*free, Some(FREE_EXPORT_NAME));

                let (size, alignment) = match element {
                    Type::Char => (1, 1), // UTF-8
                    _ => (
                        self.sizes.size(element) as u32,
                        self.sizes.align(element) as u32,
                    ),
                };

                let ptr =
                    self.emit_copy_list(size, alignment, &operands[0], &operands[1], false, None);

                results.push(Operand::List {
                    addr: ptr,
                    len: operands[1].local().unwrap(),
                });
            }
            Instruction::ListLift { element, free } => {
                // Lifting goes from adapted module to parent module
                assert_eq!(*free, Some(FREE_EXPORT_NAME));

                let offsets = offsets_for_type(interface, &self.sizes, element);
                let size = self.sizes.size(element) as u32;
                let alignment = self.sizes.align(element) as u32;

                let ptr = self.emit_copy_list(
                    size,
                    alignment,
                    &operands[0],
                    &operands[1],
                    false,
                    Some(offsets),
                );

                results.push(Operand::List {
                    addr: ptr,
                    len: operands[1].local().unwrap(),
                });
            }
            Instruction::IterElem => {
                results.push(Operand::Global(ELEMENT_GLOBAL));
            }
            Instruction::IterBasePointer => {
                results.push(Operand::GlobalPointer {
                    addr: BASE_POINTER_GLOBAL,
                    memory: 0, // Not important
                });
            }
            Instruction::BufferLowerPtrLen { .. } => todo!(),
            Instruction::BufferLowerHandle { .. } => todo!(),
            Instruction::BufferLiftPtrLen { .. } => todo!(),
            Instruction::BufferLiftHandle { .. } => todo!(),
            Instruction::RecordLower { record, .. }
            | Instruction::FlagsLower { record, .. }
            | Instruction::FlagsLower64 { record, .. } => {
                results.extend(operands.swap_remove(0).lower_record(&self.sizes, record));
            }
            Instruction::RecordLift { .. }
            | Instruction::FlagsLift { .. }
            | Instruction::FlagsLift64 { .. } => {
                results.push(Operand::Record {
                    fields: operands.into_iter().map(|o| Box::new(o.clone())).collect(),
                });
            }
            Instruction::VariantPayload => todo!(),
            Instruction::VariantLower { .. } => todo!(),
            Instruction::VariantLift { .. } => todo!(),
            Instruction::CallWasm {
                module: _,
                name: _,
                sig,
            } => {
                assert_eq!(operands.len(), sig.params.len());
                for operand in operands.iter() {
                    operand.load(self);
                }
                self.emit_instruction(wasm_encoder::Instruction::Call(self.func_index));

                if sig.retptr.is_some() {
                    assert_eq!(sig.results.len(), 1);
                    let local = self.alloc_local(sig.results[0]);
                    self.emit_instruction(wasm_encoder::Instruction::LocalSet(local));
                    results.push(Operand::Pointer {
                        addr: local,
                        memory: ADAPTED_MEMORY_INDEX,
                    });
                } else {
                    for ty in sig.results.iter() {
                        let local = self.alloc_local(*ty);
                        self.emit_instruction(wasm_encoder::Instruction::LocalSet(local));
                        results.push(Operand::Local(local));
                    }
                }
            }
            Instruction::CallInterface { .. } => todo!(),
            Instruction::Return { amt } => {
                assert!(operands.len() == *amt);

                if let Some(retptr) = self.signature.retptr.clone() {
                    let retptr_index = self.signature.params.len() as u32 - 1;
                    let mut offset = 0;
                    let mut types = retptr.iter();

                    for o in std::mem::take(operands) {
                        for local in o.locals() {
                            let ty = self.local_type(local);
                            assert_eq!(ty, *types.next().unwrap());

                            self.emit_store_from_local(
                                &Operand::Pointer {
                                    addr: retptr_index,
                                    memory: PARENT_MEMORY_INDEX,
                                },
                                offset,
                                &Operand::Local(local),
                                match ty {
                                    WasmType::I32 => StoreType::I32,
                                    WasmType::I64 => StoreType::I64,
                                    WasmType::F32 => StoreType::F32,
                                    WasmType::F64 => StoreType::F64,
                                },
                            );

                            // The return space is in 8-byte elements
                            offset += 8;
                        }
                    }

                    assert!(types.next().is_none());
                } else {
                    for operand in operands {
                        operand.load(self);
                    }
                }

                self.emit_instruction(wasm_encoder::Instruction::End);
            }
            Instruction::Witx { .. } => unreachable!(),
        }
    }

    fn allocate_typed_space(&mut self, _iface: &Interface, _ty: TypeId) -> Self::Operand {
        unreachable!("should not be called")
    }

    fn i64_return_pointer_area(&mut self, _amt: usize) -> Self::Operand {
        unreachable!("should not be called")
    }

    fn push_block(&mut self) {
        self.instructions.push(Vec::new());
    }

    fn finish_block(&mut self, operands: &mut Vec<Self::Operand>) {
        assert!(!self.instructions.is_empty());
        self.blocks
            .push((std::mem::take(operands), self.instructions.pop().unwrap()));
    }

    fn sizes(&self) -> &SizeAlign {
        &self.sizes
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::adapted::{FREE_EXPORT_NAME, REALLOC_EXPORT_NAME};
    use anyhow::Result;
    use wasm_encoder::{
        CodeSection, EntityType, FunctionSection, ImportSection, Limits, MemoryType, Module,
        TypeSection,
    };

    fn generate_adapter(interface: &str) -> Result<String> {
        let interface = Interface::parse("test", interface)?;

        let func = interface
            .functions
            .iter()
            .find(|f| f.name == "test")
            .unwrap();

        let import_signature = interface.wasm_signature(CallMode::WasmImport, func);
        let export_signature = interface.wasm_signature(CallMode::WasmExport, func);

        let mut generator = CodeGenerator::new(&interface, func, &import_signature, 0, 1, 2);

        interface.call(CallMode::WasmExport, func, &mut generator);

        let func = generator.into_function();

        let mut module = Module::new();

        let mut s = TypeSection::new();
        s.function(
            export_signature.params.iter().map(to_val_type),
            export_signature.results.iter().map(to_val_type),
        );
        s.function(
            crate::adapted::REALLOC_FUNC_TYPE
                .params
                .iter()
                .map(crate::adapted::to_val_type),
            crate::adapted::REALLOC_FUNC_TYPE
                .returns
                .iter()
                .map(crate::adapted::to_val_type),
        );
        s.function(
            crate::adapted::FREE_FUNC_TYPE
                .params
                .iter()
                .map(crate::adapted::to_val_type),
            crate::adapted::FREE_FUNC_TYPE
                .returns
                .iter()
                .map(crate::adapted::to_val_type),
        );
        s.function(
            import_signature.params.iter().map(to_val_type),
            import_signature.results.iter().map(to_val_type),
        );
        module.section(&s);

        let mut s = ImportSection::new();
        s.import("inner", Some("test"), EntityType::Function(0));
        s.import(
            "$parent",
            Some(REALLOC_EXPORT_NAME),
            EntityType::Function(1),
        );
        s.import("inner", Some(REALLOC_EXPORT_NAME), EntityType::Function(1));
        s.import("inner", Some(FREE_EXPORT_NAME), EntityType::Function(2));
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
        s.function(3);
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

    fn expected_output(export_ty: &str, import_ty: &str, func: &str) -> String {
        format!(
            "\
(module
  (type (;0;) {})
  (type (;1;) (func (param i32 i32 i32 i32) (result i32)))
  (type (;2;) (func (param i32 i32 i32)))
  (type (;3;) {})
  (import \"inner\" \"test\" (func (;0;) (type 0)))
  (import \"$parent\" \"canonical_abi_realloc\" (func (;1;) (type 1)))
  (import \"inner\" \"canonical_abi_realloc\" (func (;2;) (type 1)))
  (import \"inner\" \"canonical_abi_free\" (func (;3;) (type 2)))
  (import \"$parent\" \"memory\" (memory (;0;) 0))
  (import \"inner\" \"memory\" (memory (;1;) 0))
  {})",
            export_ty, import_ty, func
        )
    }

    #[test]
    fn generates_with_no_parameters() -> Result<()> {
        assert_eq!(
            generate_adapter("test: function()")?,
            expected_output("(func)", "(func)", "(func (;4;) (type 3)\n    call 0)")
        );

        Ok(())
    }

    #[test]
    fn generates_with_unsigned_integer_params() -> Result<()> {
        assert_eq!(
            generate_adapter("test: function(p0: u8, p1: u16, p2: u32, p3: u64)")?,
            expected_output(
                "(func (param i32 i32 i32 i64))",
                "(func (param i32 i32 i32 i64))",
                "\
(func (;4;) (type 3) (param i32 i32 i32 i64)
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
            generate_adapter("test: function(p0: s8, p1: s16, p2: s32, p3: s64)")?,
            expected_output(
                "(func (param i32 i32 i32 i64))",
                "(func (param i32 i32 i32 i64))",
                "\
(func (;4;) (type 3) (param i32 i32 i32 i64)
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
            generate_adapter("test: function(p0: f32, p1: f64)")?,
            expected_output(
                "(func (param f32 f64))",
                "(func (param f32 f64))",
                "\
(func (;4;) (type 3) (param f32 f64)
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
                generate_adapter(&format!("test: function() -> {}", ty))?,
                expected_output(
                    "(func (result i32))",
                    "(func (result i32))",
                    "\
(func (;4;) (type 3) (result i32)
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
        for ty in &["f32", "f64"] {
            assert_eq!(
                generate_adapter(&format!("test: function() -> {}", ty))?,
                expected_output(
                    &format!("(func (result {}))", ty),
                    &format!("(func (result {}))", ty),
                    &format!(
                        "\
(func (;4;) (type 3) (result {0})
    (local {0})
    call 0
    local.set 0
    local.get 0)",
                        ty
                    )
                )
            );
        }

        Ok(())
    }

    #[test]
    fn generates_with_strings() -> Result<()> {
        assert_eq!(
            generate_adapter("test: function(p0: u32, p1: string) -> string")?,
            expected_output(
                "(func (param i32 i32 i32) (result i32))",
                "(func (param i32 i32 i32 i32))",
                "\
(func (;4;) (type 3) (param i32 i32 i32 i32)
    (local i32 i32 i32 i32 i32)
    i32.const 0
    i32.const 0
    local.get 2
    i32.const 1
    call 2
    local.tee 4
    local.get 1
    local.get 2
    memory.copy 1 0
    local.get 0
    local.get 4
    local.get 2
    call 0
    local.set 5
    local.get 5
    i32.load (memory 1)
    local.set 6
    local.get 5
    i32.load (memory 1) offset=8
    local.set 7
    i32.const 0
    i32.const 0
    local.get 7
    i32.const 1
    call 1
    local.tee 8
    local.get 6
    local.get 7
    memory.copy 0 1
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
            generate_adapter("test: function(p0: s32, p1: s16) -> (s8, s64, string, f64, f32)")?,
            expected_output(
                "(func (param i32 i32) (result i32))",
                "(func (param i32 i32 i32))",
                "\
(func (;4;) (type 3) (param i32 i32 i32)
    (local i32 i32 i64 i32 i32 f64 f32 i32)
    local.get 0
    local.get 1
    call 0
    local.set 3
    local.get 3
    i32.load (memory 1)
    local.set 4
    local.get 3
    i64.load (memory 1) offset=8
    local.set 5
    local.get 3
    i32.load (memory 1) offset=16
    local.set 6
    local.get 3
    i32.load (memory 1) offset=24
    local.set 7
    local.get 3
    f64.load (memory 1) offset=32
    local.set 8
    local.get 3
    f32.load (memory 1) offset=40
    local.set 9
    i32.const 0
    i32.const 0
    local.get 7
    i32.const 1
    call 1
    local.tee 10
    local.get 6
    local.get 7
    memory.copy 0 1
    local.get 2
    local.get 4
    i32.store
    local.get 2
    local.get 5
    i64.store offset=8
    local.get 2
    local.get 10
    i32.store offset=16
    local.get 2
    local.get 7
    i32.store offset=24
    local.get 2
    local.get 8
    f64.store offset=32
    local.get 2
    local.get 9
    f32.store offset=40)"
            )
        );

        Ok(())
    }
}
