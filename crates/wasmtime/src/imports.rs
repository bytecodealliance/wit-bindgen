use crate::{BorrowHandle, GuestMemory, Region, WasmtimeGuestMemory};
use std::fmt;
use std::ops::Range;

pub struct InBuffer<'a, T> {
    mem: &'a WasmtimeGuestMemory<'a>,
    offset: i32,
    len: i32,
    size: i32,
    deserialize: &'a (dyn Fn(i32) -> Result<T, wasmtime::Trap> + 'a),
}

impl<'a, T> InBuffer<'a, T> {
    pub fn new(
        mem: &'a WasmtimeGuestMemory<'a>,
        offset: i32,
        len: i32,
        size: i32,
        deserialize: &'a (dyn Fn(i32) -> Result<T, wasmtime::Trap> + 'a),
    ) -> InBuffer<'a, T> {
        InBuffer {
            mem,
            offset,
            len,
            size,
            deserialize,
        }
    }

    pub fn len(&self) -> usize {
        self.len as u32 as usize
    }

    pub fn iter<'b>(&'b self) -> Result<InBufferIter<'a, 'b, T>, wasmtime::Trap> {
        let region = Region {
            start: self.offset as u32,
            len: (self.len as u32)
                .checked_mul(self.size as u32)
                .ok_or_else(|| wasmtime::Trap::new("length overflow"))?,
        };
        let borrow = self
            .mem
            .shared_borrow(region)
            .map_err(|e| wasmtime::Trap::new(format!("borrow error: {}", e)))?;
        Ok(InBufferIter {
            buffer: self,
            range: 0..self.len,
            borrow,
        })
    }
}

impl<T> fmt::Debug for InBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InBuffer")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .finish()
    }
}

pub struct InBufferIter<'a, 'b, T> {
    buffer: &'b InBuffer<'a, T>,
    range: Range<i32>,
    borrow: BorrowHandle,
}

impl<T> Iterator for InBufferIter<'_, '_, T> {
    type Item = Result<T, wasmtime::Trap>;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.range.next()?;
        Some((self.buffer.deserialize)(
            self.buffer.offset + i * self.buffer.size,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<T> Drop for InBufferIter<'_, '_, T> {
    fn drop(&mut self) {
        self.buffer.mem.shared_unborrow(self.borrow);
    }
}

pub struct OutBuffer<'a, T> {
    mem: &'a WasmtimeGuestMemory<'a>,
    offset: i32,
    len: i32,
    size: i32,
    serialize: &'a (dyn Fn(i32, T) -> Result<(), wasmtime::Trap> + 'a),
}

impl<'a, T> OutBuffer<'a, T> {
    pub fn new(
        mem: &'a WasmtimeGuestMemory<'a>,
        offset: i32,
        len: i32,
        size: i32,
        serialize: &'a (dyn Fn(i32, T) -> Result<(), wasmtime::Trap> + 'a),
    ) -> OutBuffer<'a, T> {
        OutBuffer {
            mem,
            offset,
            len,
            size,
            serialize,
        }
    }

    pub fn capacity(&self) -> usize {
        self.len as u32 as usize
    }

    pub fn write(&mut self, iter: impl IntoIterator<Item = T>) -> Result<(), wasmtime::Trap> {
        let region = Region {
            start: self.offset as u32,
            len: (self.len as u32)
                .checked_mul(self.size as u32)
                .ok_or_else(|| wasmtime::Trap::new("length overflow"))?,
        };
        let borrow = self
            .mem
            .mut_borrow(region)
            .map_err(|e| wasmtime::Trap::new(format!("borrow error: {}", e)))?;
        for item in iter {
            if self.len == 0 {
                self.mem.mut_unborrow(borrow);
                return Err(wasmtime::Trap::new(
                    "too many results in `OutBuffer::write`",
                ));
            }
            (self.serialize)(self.offset, item)?;
            self.len -= 1;
            self.offset += self.size;
        }
        self.mem.mut_unborrow(borrow);
        Ok(())
    }
}

impl<T> fmt::Debug for OutBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutBuffer")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .finish()
    }
}
