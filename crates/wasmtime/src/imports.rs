use crate::BorrowChecker;
use std::fmt;
use std::mem;
use wasmtime::Trap;

pub struct InBuffer<'a, T> {
    mem: &'a [u8],
    size: usize,
    deserialize: &'a (dyn Fn(&'a [u8]) -> Result<T, Trap> + 'a),
}

impl<'a, T> InBuffer<'a, T> {
    pub fn new(
        mem: &mut BorrowChecker<'a>,
        offset: i32,
        len: i32,
        size: i32,
        deserialize: &'a (dyn Fn(&'a [u8]) -> Result<T, Trap> + 'a),
    ) -> Result<InBuffer<'a, T>, Trap> {
        Ok(InBuffer {
            mem: unsafe { mem.slice(offset, len.saturating_mul(size))? },
            size: size as usize,
            deserialize,
        })
    }

    pub fn len(&self) -> usize {
        self.mem.len() / self.size
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<T, Trap>> + 'a {
        let deserialize = self.deserialize;
        self.mem.chunks(self.size).map(deserialize)
    }
}

impl<T> fmt::Debug for InBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InBuffer")
            .field("len", &self.len())
            .finish()
    }
}

pub struct OutBuffer<'a, T> {
    mem: &'a mut [u8],
    size: usize,
    serialize: &'a (dyn Fn(&mut [u8], T) -> Result<(), Trap> + 'a),
}

impl<'a, T> OutBuffer<'a, T> {
    pub fn new(
        mem: &mut BorrowChecker<'a>,
        offset: i32,
        len: i32,
        size: i32,
        serialize: &'a (dyn Fn(&mut [u8], T) -> Result<(), Trap> + 'a),
    ) -> Result<OutBuffer<'a, T>, Trap> {
        let mem =
            unsafe { mem.slice_mut(offset, (len as u32).saturating_mul(size as u32) as i32)? };
        Ok(OutBuffer {
            mem,
            size: size as usize,
            serialize,
        })
    }

    pub fn capacity(&self) -> usize {
        self.mem.len() / self.size
    }

    pub fn write(&mut self, iter: impl IntoIterator<Item = T>) -> Result<(), Trap> {
        for item in iter {
            if self.mem.len() == 0 {
                return Err(Trap::new("too many results in `OutBuffer::write`"));
            }
            let (chunk, rest) = mem::take(&mut self.mem).split_at_mut(self.size);
            self.mem = rest;
            (self.serialize)(chunk, item)?;
        }
        Ok(())
    }
}

impl<T> fmt::Debug for OutBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutBuffer")
            .field("capacity", &self.capacity())
            .finish()
    }
}
