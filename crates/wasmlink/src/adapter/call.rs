use crate::module::Interface;
use std::collections::HashMap;
use wasm_encoder::{BlockType, Instruction, MemArg, ValType};
use wit_parser::{
    abi::WasmSignature, Function, Int, Interface as WitInterface, RecordKind, SizeAlign, Type,
    TypeDefKind,
};

// The parent's memory is imported, so it is always index 0 for the adapter logic
const PARENT_MEMORY_INDEX: u32 = 0;
// The adapted module's memory is aliased, so it is always index 1 for the adapter logic
pub const ADAPTED_MEMORY_INDEX: u32 = 1;

struct Locals {
    start: u32,
    count: u32,
    allocated: u32,
    map: HashMap<u32, u32>,
}

impl Locals {
    fn new(start: u32, count: u32) -> Self {
        Self {
            start,
            count,
            allocated: 0,
            map: HashMap::new(),
        }
    }

    fn allocate(&mut self) -> u32 {
        assert!(self.allocated < self.count);
        let index = self.start + self.allocated;
        self.allocated += 1;
        index
    }

    fn map(&mut self, old: u32) -> u32 {
        let new = self.allocate();
        self.map.insert(old, new);
        new
    }

    fn lookup(&self, old: u32) -> Option<u32> {
        self.map.get(&old).copied()
    }

    fn take(&mut self, old: u32) -> u32 {
        self.map.remove(&old).unwrap()
    }
}

#[derive(Debug, Copy, Clone)]
enum Direction {
    /// Copying data from parent module to adapted module.
    In,
    /// Copying data from adapted module to parent module.
    Out,
}

#[derive(Debug, Copy, Clone)]
struct ElementBase {
    /// The local storing the list's base address.
    base: u32,
    /// If some, the pair of the local storing the current index and the element's size.
    /// If none, the list's base address is used.
    index_and_size: Option<(u32, u32)>,
    /// The memory index for the list.
    memory: u32,
}

#[derive(Debug, Copy, Clone)]
enum LoadType {
    I32_8U,
    I32_16U,
    I32,
    I64,
}

impl From<Int> for LoadType {
    fn from(i: Int) -> Self {
        match i {
            Int::U8 => Self::I32_8U,
            Int::U16 => Self::I32_16U,
            Int::U32 => Self::I32,
            Int::U64 => Self::I64,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum ValueRef {
    /// A reference to a 32-bit value stored in a function local (by index).
    Local(u32),
    /// A reference to a 32-bit value via an offset from an element base address.
    ElementOffset(u32),
    /// A reference to a 32-bit value returned from a function stored in a local (single-value return).
    Return(u32),
}

impl ValueRef {
    fn offset(&self) -> Option<u32> {
        match self {
            ValueRef::Local(_) | ValueRef::Return(_) => None,
            ValueRef::ElementOffset(o) => Some(*o),
        }
    }

    fn emit_load(
        &self,
        function: &mut wasm_encoder::Function,
        base: Option<ElementBase>,
        ty: LoadType,
    ) {
        match self {
            ValueRef::Local(i) | ValueRef::Return(i) => {
                function.instruction(&Instruction::LocalGet(*i));
                return;
            }
            ValueRef::ElementOffset(_) => {}
        }

        let base = base.expect("cannot load via an element offset without a base");
        function.instruction(&Instruction::LocalGet(base.base));
        if let Some((index, size)) = base.index_and_size {
            function.instruction(&Instruction::LocalGet(index));
            function.instruction(&Instruction::I32Const(size as i32));
            function.instruction(&Instruction::I32Mul);
            function.instruction(&Instruction::I32Add);
        }

        let memarg = MemArg {
            offset: self.offset().expect("reference should have an offset") as u64,
            align: match ty {
                LoadType::I32_8U => 0,
                LoadType::I32_16U => 1,
                LoadType::I32 => 2,
                LoadType::I64 => 3,
            },
            memory_index: base.memory,
        };

        match ty {
            LoadType::I32_8U => function.instruction(&Instruction::I32Load8_U(memarg)),
            LoadType::I32_16U => function.instruction(&Instruction::I32Load16_U(memarg)),
            LoadType::I32 => function.instruction(&Instruction::I32Load(memarg)),
            LoadType::I64 => function.instruction(&Instruction::I64Load(memarg)),
        };
    }
}

#[derive(Debug)]
enum Operand<'a> {
    Variant {
        discriminant: (ValueRef, LoadType),
        cases: Vec<(u32, Vec<Operand<'a>>)>,
    },
    List {
        addr: ValueRef,
        len: ValueRef,
        element_size: u32,
        element_alignment: u32,
        operands: Vec<Operand<'a>>,
    },
    Handle {
        addr: ValueRef,
        name: &'a str,
    },
}

impl Operand<'_> {
    fn local(&self) -> Option<u32> {
        match self {
            Operand::List { addr, .. } | Operand::Handle { addr, .. } => match addr {
                ValueRef::Local(i) => Some(*i),
                _ => None,
            },
            Operand::Variant { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PushMode {
    Params,
    Return,
}

impl PushMode {
    fn create_value_ref(self, val: u32) -> ValueRef {
        match self {
            Self::Params => ValueRef::Local(val),
            Self::Return => ValueRef::Return(val),
        }
    }
}

#[derive(Debug)]
pub(crate) struct CallAdapter<'a> {
    signature: &'a WasmSignature,
    locals_count: u32,
    params: Vec<Operand<'a>>,
    results: Vec<Operand<'a>>,
    call_index: u32,
    realloc_index: Option<u32>,
    free_index: Option<u32>,
    parent_realloc_index: Option<u32>,
    resource_functions: &'a HashMap<&'a str, (u32, u32)>,
    result_size: usize,
}

impl<'a> CallAdapter<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interface: &'a Interface,
        signature: &'a WasmSignature,
        func: &Function,
        call_index: u32,
        realloc_index: Option<u32>,
        free_index: Option<u32>,
        parent_realloc_index: Option<u32>,
        resource_functions: &'a HashMap<&'a str, (u32, u32)>,
    ) -> Self {
        let inner = interface.inner();
        let sizes = interface.sizes();

        let mut locals_count = 0;

        let mut iter = 0..signature.params.len() as u32;
        let mut params = Vec::new();
        for (_, ty) in &func.params {
            Self::push_operands(
                inner,
                sizes,
                ty,
                &mut iter,
                PushMode::Params,
                &mut locals_count,
                &mut params,
            );
        }

        let results = if signature.retptr {
            // For the callee's retptr
            locals_count += 1;

            let mut results = Vec::new();
            Self::push_element_operands(
                inner,
                sizes,
                &func.result,
                0,
                PushMode::Return,
                &mut locals_count,
                &mut results,
            );

            results
        } else {
            // Use the possible index for the return value local
            let index = signature.params.len() as u32 + locals_count;

            let mut iter = index..index + 1;
            let mut results = Vec::new();
            Self::push_operands(
                inner,
                sizes,
                &func.result,
                &mut iter,
                PushMode::Return,
                &mut locals_count,
                &mut results,
            );

            if !results.is_empty() {
                // There's an operand to copy, so we'll need a local for the return value
                locals_count += 1;
            }

            results
        };

        Self {
            signature,
            locals_count,
            params,
            results,
            call_index,
            realloc_index,
            free_index,
            parent_realloc_index,
            resource_functions,
            result_size: sizes.size(&func.result),
        }
    }

    pub fn adapt(&self) -> wasm_encoder::Function {
        let mut locals = Locals::new(self.signature.params.len() as u32, self.locals_count);
        let mut function = wasm_encoder::Function::new([(self.locals_count, ValType::I32)]);

        self.copy_parameters(&mut function, &mut locals);
        self.emit_call(&mut function, &locals);
        self.copy_results(&mut function, &mut locals);

        assert_eq!(locals.allocated, locals.count);

        function.instruction(&Instruction::End);

        function
    }

    fn copy_parameters(&self, function: &mut wasm_encoder::Function, locals: &mut Locals) {
        for param in &self.params {
            self.emit_copy_operand(function, locals, Direction::In, param, None, None);
        }
    }

    fn emit_call(&self, function: &mut wasm_encoder::Function, locals: &Locals) {
        let params = if self.signature.retptr {
            self.signature.params.len() as u32 - 1
        } else {
            self.signature.params.len() as u32
        };

        for i in 0..params {
            function.instruction(&Instruction::LocalGet(locals.lookup(i).unwrap_or(i)));
        }

        function.instruction(&Instruction::Call(self.call_index));
    }

    fn copy_results(&self, function: &mut wasm_encoder::Function, locals: &mut Locals) {
        if self.signature.retptr {
            let src_retptr = locals.allocate();
            let dst_retptr = self.signature.params.len() as u32 - 1;
            function.instruction(&Instruction::LocalSet(src_retptr));

            if self.result_size > 0 {
                function.instruction(&Instruction::LocalGet(dst_retptr));
                function.instruction(&Instruction::LocalGet(src_retptr));
                function.instruction(&Instruction::I32Const(self.result_size as i32));
                function.instruction(&Instruction::MemoryCopy {
                    src: ADAPTED_MEMORY_INDEX,
                    dst: PARENT_MEMORY_INDEX,
                });
            }

            let src_base = ElementBase {
                base: src_retptr,
                index_and_size: None,
                memory: ADAPTED_MEMORY_INDEX,
            };

            let dst_base = ElementBase {
                base: dst_retptr,
                index_and_size: None,
                memory: PARENT_MEMORY_INDEX,
            };

            for result in &self.results {
                self.emit_copy_operand(
                    function,
                    locals,
                    Direction::Out,
                    result,
                    Some(src_base),
                    Some(dst_base),
                );
            }

            return;
        }

        if self.results.is_empty() {
            return;
        }

        assert_eq!(self.results.len(), 1);

        let src = locals.allocate();

        // Store the value returned from the call
        function.instruction(&Instruction::LocalSet(src));

        self.emit_copy_operand(
            function,
            locals,
            Direction::Out,
            &self.results[0],
            None,
            None,
        );

        function.instruction(&Instruction::LocalGet(locals.lookup(src).unwrap_or(src)));
    }

    fn push_operands<T>(
        interface: &'a WitInterface,
        sizes: &SizeAlign,
        ty: &Type,
        params: &mut T,
        mode: PushMode,
        locals_count: &mut u32,
        operands: &mut Vec<Operand<'a>>,
    ) where
        T: ExactSizeIterator<Item = u32> + Clone,
    {
        match ty {
            Type::Id(id) => match &interface.types[*id].kind {
                TypeDefKind::Type(t) => {
                    Self::push_operands(interface, sizes, t, params, mode, locals_count, operands)
                }
                TypeDefKind::List(element) => {
                    let addr = params.next().unwrap();
                    let len = params.next().unwrap();

                    let mut element_operands = Vec::new();
                    if !interface.all_bits_valid(element) {
                        Self::push_element_operands(
                            interface,
                            sizes,
                            element,
                            0,
                            mode,
                            locals_count,
                            &mut element_operands,
                        );
                    }

                    // Every list copied needs a destination local (and a source for retptr)
                    *locals_count += match mode {
                        PushMode::Params => 1,
                        PushMode::Return => unreachable!(),
                    };

                    // Lists that copy elements with lists need a local for the counter
                    if !element_operands.is_empty() {
                        *locals_count += 1;
                    }

                    let (element_size, element_alignment) = match element {
                        Type::Char => (1, 1), // UTF-8
                        _ => (sizes.size(element) as u32, sizes.align(element) as u32),
                    };

                    operands.push(Operand::List {
                        addr: mode.create_value_ref(addr),
                        len: mode.create_value_ref(len),
                        element_size,
                        element_alignment,
                        operands: element_operands,
                    });
                }
                TypeDefKind::Record(r) => match r.kind {
                    RecordKind::Flags(_) => match interface.flags_repr(r) {
                        Some(_) => {
                            params.next().unwrap();
                        }
                        None => {
                            for _ in 0..r.num_i32s() {
                                params.next().unwrap();
                            }
                        }
                    },
                    RecordKind::Tuple | RecordKind::Other => {
                        for f in &r.fields {
                            Self::push_operands(
                                interface,
                                sizes,
                                &f.ty,
                                params,
                                mode,
                                locals_count,
                                operands,
                            );
                        }
                    }
                },
                TypeDefKind::Variant(v) if v.is_enum() => {
                    params.next().unwrap();
                }
                TypeDefKind::Variant(v) => {
                    let discriminant = params.next().unwrap();
                    let mut count = 0;
                    let mut cases = Vec::new();
                    for (i, c) in v.cases.iter().enumerate() {
                        if let Some(ty) = &c.ty {
                            let mut iter = params.clone();
                            let mut operands = Vec::new();

                            Self::push_operands(
                                interface,
                                sizes,
                                ty,
                                &mut iter,
                                mode,
                                locals_count,
                                &mut operands,
                            );

                            if !operands.is_empty() {
                                cases.push((i as u32, operands));
                            }

                            count = std::cmp::max(count, params.len() - iter.len());
                        }
                    }

                    if !cases.is_empty() {
                        operands.push(Operand::Variant {
                            discriminant: (mode.create_value_ref(discriminant), v.tag.into()),
                            cases,
                        });
                    }

                    for _ in 0..count {
                        params.next().unwrap();
                    }
                }
            },
            Type::String => {
                let addr = params.next().unwrap();
                let len = params.next().unwrap();

                // Every list copied needs a destination local (and a source for
                // retptr)
                *locals_count += match mode {
                    PushMode::Params => 1,
                    PushMode::Return => unreachable!(),
                };

                operands.push(Operand::List {
                    addr: mode.create_value_ref(addr),
                    len: mode.create_value_ref(len),
                    element_size: 1, // UTF-8
                    element_alignment: 1,
                    operands: Vec::new(),
                });
            }
            Type::Handle(id) => {
                let addr = params.next().unwrap();

                // Params need to be cloned, so add a local
                *locals_count += match mode {
                    PushMode::Params => 1,
                    PushMode::Return => 0,
                };

                operands.push(Operand::Handle {
                    addr: mode.create_value_ref(addr),
                    name: interface.resources[*id].name.as_str(),
                });
            }
            _ => {
                params.next().unwrap();
            }
        }
    }

    fn push_element_operands(
        interface: &'a WitInterface,
        sizes: &SizeAlign,
        ty: &Type,
        offset: u32,
        mode: PushMode,
        locals_count: &mut u32,
        operands: &mut Vec<Operand<'a>>,
    ) {
        match ty {
            Type::Id(id) => match &interface.types[*id].kind {
                TypeDefKind::Type(t) => Self::push_element_operands(
                    interface,
                    sizes,
                    t,
                    offset,
                    mode,
                    locals_count,
                    operands,
                ),
                TypeDefKind::List(element) => {
                    let mut element_operands = Vec::new();
                    if !interface.all_bits_valid(element) {
                        Self::push_element_operands(
                            interface,
                            sizes,
                            element,
                            0,
                            mode,
                            locals_count,
                            &mut element_operands,
                        );
                    }

                    // Every list copied needs a source and destination local
                    *locals_count += 2;

                    // Lists with elements containing lists need a local for the loop counter
                    if !element_operands.is_empty() {
                        *locals_count += 1;
                    }

                    let (element_size, element_alignment) = match element {
                        Type::Char => (1, 1), // UTF-8
                        _ => (sizes.size(element) as u32, sizes.align(element) as u32),
                    };

                    operands.push(Operand::List {
                        addr: ValueRef::ElementOffset(offset),
                        len: ValueRef::ElementOffset(offset + 4),
                        element_size,
                        element_alignment,
                        operands: element_operands,
                    });
                }
                TypeDefKind::Record(r) => match r.kind {
                    RecordKind::Flags(_) => {}
                    RecordKind::Tuple | RecordKind::Other => {
                        let offsets = sizes.field_offsets(r);

                        for (f, o) in r.fields.iter().zip(offsets) {
                            Self::push_element_operands(
                                interface,
                                sizes,
                                &f.ty,
                                offset + o as u32,
                                mode,
                                locals_count,
                                operands,
                            );
                        }
                    }
                },
                TypeDefKind::Variant(v) if v.is_enum() => {}
                TypeDefKind::Variant(v) => {
                    let payload_offset = sizes.payload_offset(v) as u32;

                    let mut cases = Vec::new();
                    for (i, c) in v.cases.iter().enumerate() {
                        if let Some(ty) = &c.ty {
                            let mut operands = Vec::new();
                            Self::push_element_operands(
                                interface,
                                sizes,
                                ty,
                                offset + payload_offset,
                                mode,
                                locals_count,
                                &mut operands,
                            );
                            if !operands.is_empty() {
                                cases.push((i as u32, operands));
                            }
                        }
                    }

                    if !cases.is_empty() {
                        operands.push(Operand::Variant {
                            discriminant: (ValueRef::ElementOffset(offset), v.tag.into()),
                            cases,
                        });
                    }
                }
            },
            Type::String => {
                // Every list copied needs a source and destination local
                *locals_count += 2;

                operands.push(Operand::List {
                    addr: ValueRef::ElementOffset(offset),
                    len: ValueRef::ElementOffset(offset + 4),
                    element_size: 1, // UTF-8
                    element_alignment: 1,
                    operands: Vec::new(),
                });
            }
            Type::Handle(id) => {
                // Params need to be cloned, so add a local
                *locals_count += match mode {
                    PushMode::Params => 1,
                    PushMode::Return => 0,
                };

                operands.push(Operand::Handle {
                    addr: ValueRef::ElementOffset(offset),
                    name: interface.resources[*id].name.as_str(),
                });
            }
            _ => {}
        }
    }

    fn emit_store_from_base(
        &self,
        function: &mut wasm_encoder::Function,
        base: &ElementBase,
        offset: u64,
        value: u32,
    ) {
        function.instruction(&Instruction::LocalGet(base.base));
        if let Some((index, size)) = base.index_and_size {
            function.instruction(&Instruction::LocalGet(index));
            function.instruction(&Instruction::I32Const(size as i32));
            function.instruction(&Instruction::I32Mul);
            function.instruction(&Instruction::I32Add);
        }
        function.instruction(&Instruction::LocalGet(value));
        function.instruction(&Instruction::I32Store(MemArg {
            offset,
            align: 2,
            memory_index: base.memory,
        }));
    }

    fn emit_copy_operand(
        &self,
        function: &mut wasm_encoder::Function,
        locals: &mut Locals,
        direction: Direction,
        operand: &'a Operand,
        src_base: Option<ElementBase>,
        dst_base: Option<ElementBase>,
    ) {
        match operand {
            Operand::Variant {
                discriminant,
                cases,
            } => {
                let (discriminant, ty) = discriminant;

                for (case, operands) in cases {
                    function.instruction(&Instruction::Block(BlockType::Empty));

                    discriminant.emit_load(function, src_base, *ty);
                    function.instruction(&Instruction::I32Const(*case as i32));
                    function.instruction(&Instruction::I32Ne);
                    function.instruction(&Instruction::BrIf(0));

                    for operand in operands {
                        self.emit_copy_operand(
                            function, locals, direction, operand, src_base, dst_base,
                        );

                        // If the operand had a local value, take from the map and assign it here as part of the
                        // variant's block.
                        if let Some(i) = operand.local() {
                            function.instruction(&Instruction::LocalGet(locals.take(i)));
                            function.instruction(&Instruction::LocalSet(i));
                        }
                    }

                    function.instruction(&Instruction::End);
                }
            }
            Operand::List {
                addr,
                len,
                element_size,
                element_alignment,
                operands,
            } => {
                let (src_list, dst_list) = self.emit_copy_list(
                    function,
                    locals,
                    direction,
                    *addr,
                    *len,
                    src_base,
                    *element_size,
                    *element_alignment,
                );

                // Now that the list has been copied, update the element in the parent list
                if let Some(offset) = addr.offset() {
                    self.emit_store_from_base(
                        function,
                        &dst_base.expect("destination base should be present"),
                        offset as u64,
                        dst_list,
                    );
                }

                if !operands.is_empty() {
                    self.emit_copy_element_operands(
                        function,
                        locals,
                        direction,
                        *len,
                        src_base,
                        *element_size,
                        src_list,
                        dst_list,
                        operands,
                    );
                }

                // Free the source list for returned values
                if let Direction::Out = direction {
                    addr.emit_load(function, src_base, LoadType::I32);
                    len.emit_load(function, src_base, LoadType::I32);
                    if *element_size > 1 {
                        function.instruction(&Instruction::I32Const(*element_size as i32));
                        function.instruction(&Instruction::I32Mul);
                    }
                    function.instruction(&Instruction::I32Const(*element_alignment as i32));
                    function.instruction(&Instruction::Call(
                        self.free_index
                            .expect("must be given an index to copy lists"),
                    ));
                }
            }
            Operand::Handle { addr, name } => {
                addr.emit_load(function, src_base, LoadType::I32);

                let (clone_func_index, get_func_index) = self.resource_functions[name];

                match direction {
                    Direction::In => {
                        // For resources being passed to a function, the callee owns
                        // the handle, so do a clone
                        function.instruction(&Instruction::Call(clone_func_index));

                        let dst = match addr {
                            ValueRef::Local(i) | ValueRef::Return(i) => locals.map(*i),
                            ValueRef::ElementOffset(_) => locals.allocate(),
                        };

                        function.instruction(&Instruction::LocalSet(dst));

                        // Now that the handle has been cloned, update the element in the parent list
                        if let Some(offset) = addr.offset() {
                            self.emit_store_from_base(
                                function,
                                &dst_base.expect("destination base should be present"),
                                offset as u64,
                                dst,
                            );
                        }
                    }
                    Direction::Out => {
                        // For returned handles, ownership is transferring to the caller, so just validate
                        // that the handle is valid with a call to get the underlying resource
                        function.instruction(&Instruction::Call(get_func_index));
                        function.instruction(&Instruction::Drop);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_copy_element_operands(
        &self,
        function: &mut wasm_encoder::Function,
        locals: &mut Locals,
        direction: Direction,
        len: ValueRef,
        len_base: Option<ElementBase>,
        element_size: u32,
        src_list: u32,
        dst_list: u32,
        operands: &[Operand],
    ) {
        let index = locals.allocate();

        let (src_memory, dst_memory) = match direction {
            Direction::In => (PARENT_MEMORY_INDEX, ADAPTED_MEMORY_INDEX),
            Direction::Out => (ADAPTED_MEMORY_INDEX, PARENT_MEMORY_INDEX),
        };

        let src_base = ElementBase {
            base: src_list,
            index_and_size: Some((index, element_size)),
            memory: src_memory,
        };

        let dst_base = ElementBase {
            base: dst_list,
            index_and_size: Some((index, element_size)),
            memory: dst_memory,
        };

        function.instruction(&Instruction::I32Const(0));
        function.instruction(&Instruction::LocalSet(index));

        function.instruction(&Instruction::Block(BlockType::Empty));
        function.instruction(&Instruction::Loop(BlockType::Empty));

        len.emit_load(function, len_base, LoadType::I32);

        function.instruction(&Instruction::LocalGet(index));
        function.instruction(&Instruction::I32Eq);
        function.instruction(&Instruction::BrIf(1));

        for operand in operands {
            self.emit_copy_operand(
                function,
                locals,
                direction,
                operand,
                Some(src_base),
                Some(dst_base),
            );
        }

        function.instruction(&Instruction::LocalGet(index));
        function.instruction(&Instruction::I32Const(1));
        function.instruction(&Instruction::I32Add);
        function.instruction(&Instruction::LocalSet(index));

        function.instruction(&Instruction::Br(0));
        function.instruction(&Instruction::End);
        function.instruction(&Instruction::End);
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_copy_list(
        &self,
        function: &mut wasm_encoder::Function,
        locals: &mut Locals,
        direction: Direction,
        addr: ValueRef,
        len: ValueRef,
        src_base: Option<ElementBase>,
        element_size: u32,
        element_alignment: u32,
    ) -> (u32, u32) {
        let (src_memory, dst_memory, realloc) = match direction {
            Direction::In => (
                PARENT_MEMORY_INDEX,
                ADAPTED_MEMORY_INDEX,
                self.realloc_index
                    .expect("must be given an index to copy lists"),
            ),
            Direction::Out => (
                ADAPTED_MEMORY_INDEX,
                PARENT_MEMORY_INDEX,
                self.parent_realloc_index
                    .expect("must be given an index to copy lists"),
            ),
        };

        let (src, dst) = match addr {
            ValueRef::Local(i) => (i, locals.map(i)),
            ValueRef::ElementOffset(_) => {
                let src = locals.allocate();
                addr.emit_load(function, src_base, LoadType::I32);
                function.instruction(&Instruction::LocalSet(src));
                (src, locals.allocate())
            }
            ValueRef::Return(_) => unreachable!(),
        };

        function.instruction(&Instruction::Block(BlockType::Empty));
        function.instruction(&Instruction::I32Const(0)); // Previous ptr
        function.instruction(&Instruction::I32Const(0)); // Previous size
        function.instruction(&Instruction::I32Const(element_alignment as i32));
        len.emit_load(function, src_base, LoadType::I32);
        if element_size > 1 {
            function.instruction(&Instruction::I32Const(element_size as i32));
            function.instruction(&Instruction::I32Mul);
        }
        function.instruction(&Instruction::Call(realloc));
        function.instruction(&Instruction::LocalTee(dst));
        function.instruction(&Instruction::BrIf(0));
        function.instruction(&Instruction::Unreachable);
        function.instruction(&Instruction::End);
        function.instruction(&Instruction::LocalGet(dst));
        function.instruction(&Instruction::LocalGet(src));
        len.emit_load(function, src_base, LoadType::I32);
        if element_size > 1 {
            function.instruction(&Instruction::I32Const(element_size as i32));
            function.instruction(&Instruction::I32Mul);
        }
        function.instruction(&Instruction::MemoryCopy {
            src: src_memory,
            dst: dst_memory,
        });

        (src, dst)
    }
}
