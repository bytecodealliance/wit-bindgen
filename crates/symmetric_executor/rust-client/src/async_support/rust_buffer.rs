// unlike abi_buffer this caches high level elements as we directly write into the
// read buffer and don't need a storage for lowered elements

use std::collections::VecDeque;

pub struct RustBuffer<T: 'static> {
    buf: VecDeque<T>,
}

// struct BufIterator<'a, T: 'static> {
//     buf: &'a mut RustBuffer<T>,
// }

impl<T: 'static> RustBuffer<T> {
    pub(crate) fn new(vec: Vec<T>) -> Self {
        Self { buf: vec.into() }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn drain_n(&mut self, n: usize) -> impl Iterator<Item = T> + use<'_, T> {
        self.buf.drain(0..n)
    }

    pub(crate) fn into_vec(&mut self) -> Vec<T> {
        todo!()
    }
}
