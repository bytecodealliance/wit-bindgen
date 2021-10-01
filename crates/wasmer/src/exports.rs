use crate::BorrowChecker;
use std::fmt;
use std::mem;
use wasmer::RuntimeError;

pub struct PullBuffer<'a, T> {
    mem: &'a [u8],
    size: usize,
    deserialize: &'a (dyn Fn(&'a [u8]) -> Result<T, RuntimeError> + Send + Sync + 'a),
}

impl<'a, T> PullBuffer<'a, T> {
    pub fn new(
        mem: &mut BorrowChecker<'a>,
        offset: i32,
        len: i32,
        size: i32,
        deserialize: &'a (dyn Fn(&'a [u8]) -> Result<T, RuntimeError> + Send + Sync + 'a),
    ) -> Result<PullBuffer<'a, T>, RuntimeError> {
        Ok(PullBuffer {
            mem: mem.slice(offset, len.saturating_mul(size))?,
            size: size as usize,
            deserialize,
        })
    }

    pub fn len(&self) -> usize {
        self.mem.len() / self.size
    }

    pub fn iter(&self) -> impl Iterator<Item = Result<T, RuntimeError>> + 'a {
        let deserialize = self.deserialize;
        self.mem.chunks(self.size).map(deserialize)
    }
}

impl<T> fmt::Debug for PullBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PullBuffer")
            .field("len", &self.len())
            .finish()
    }
}

pub struct PushBuffer<'a, T> {
    mem: &'a mut [u8],
    size: usize,
    serialize: &'a (dyn Fn(&mut [u8], T) -> Result<(), RuntimeError> + Send + Sync + 'a),
}

impl<'a, T> PushBuffer<'a, T> {
    pub fn new(
        mem: &mut BorrowChecker<'a>,
        offset: i32,
        len: i32,
        size: i32,
        serialize: &'a (dyn Fn(&mut [u8], T) -> Result<(), RuntimeError> + Send + Sync + 'a),
    ) -> Result<PushBuffer<'a, T>, RuntimeError> {
        let mem = mem.slice_mut(offset, (len as u32).saturating_mul(size as u32) as i32)?;
        Ok(PushBuffer {
            mem,
            size: size as usize,
            serialize,
        })
    }

    pub fn capacity(&self) -> usize {
        self.mem.len() / self.size
    }

    pub fn write(&mut self, iter: impl IntoIterator<Item = T>) -> Result<(), RuntimeError> {
        for item in iter {
            if self.mem.len() == 0 {
                return Err(RuntimeError::new("too many results in `PushBuffer::write`"));
            }
            let (chunk, rest) = mem::take(&mut self.mem).split_at_mut(self.size);
            self.mem = rest;
            (self.serialize)(chunk, item)?;
        }
        Ok(())
    }
}

impl<T> fmt::Debug for PushBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PushBuffer")
            .field("capacity", &self.capacity())
            .finish()
    }
}
