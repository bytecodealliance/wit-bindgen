//! Assigning static locations for data segments we will emit in the glue Wasm
//! module.

use std::convert::TryFrom;

#[derive(Debug)]
pub struct DataSegments {
    data: wasm_encoder::DataSection,
    next_offset: u32,
    memory: u32,
}

impl DataSegments {
    /// Create a new collection of data segments for the given memory.
    pub fn new(memory: u32) -> DataSegments {
        DataSegments {
            data: wasm_encoder::DataSection::new(),
            next_offset: 0,
            memory,
        }
    }

    /// Add a new segment to this `DataSegments`, returning the assigned offset
    /// in memory.
    pub fn add<S>(&mut self, segment: S) -> u32
    where
        S: IntoIterator<Item = u8>,
        S::IntoIter: ExactSizeIterator,
    {
        let segment = segment.into_iter();
        let offset = self.reserve_space(u32::try_from(segment.len()).unwrap());
        self.data.active(
            self.memory,
            &wasm_encoder::Instruction::I32Const(offset as i32),
            segment,
        );
        offset
    }

    /// Reserve space in memory but don't emit any data segment to initialize
    /// it.
    ///
    /// This effectively lets you add zero-initialized data segments, reserve
    /// space for return pointer areas, or define shadow stack regions.
    pub fn reserve_space(&mut self, num_bytes: u32) -> u32 {
        // Leave an empty byte between each data segment. This helps when
        // staring at disassemblies and heap dumps.
        self.next_offset += 1;

        let offset = self.next_offset;
        self.next_offset += num_bytes;

        offset
    }

    /// Get the memory type required to hold these data segments.
    pub fn memory_type(&self) -> wasm_encoder::MemoryType {
        const WASM_PAGE_SIZE: u32 = 65_536;
        wasm_encoder::MemoryType {
            minimum: ((self.next_offset + WASM_PAGE_SIZE - 1) / WASM_PAGE_SIZE).into(),
            maximum: None,
            memory64: false,
        }
    }

    /// Take the constructed data section.
    ///
    /// No more data segments should be added after this is called.
    pub fn take_data(&mut self) -> wasm_encoder::DataSection {
        std::mem::replace(&mut self.data, wasm_encoder::DataSection::new())
    }
}
