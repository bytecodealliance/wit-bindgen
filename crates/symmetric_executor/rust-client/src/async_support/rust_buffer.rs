// unlike abi_buffer this caches high level elements as we directly write into the
// read buffer and don't need a storage for lowered elements

use std::collections::VecDeque;

pub struct RustBuffer<T: 'static> {
    buf: VecDeque<T>,
}

impl<T: 'static> RustBuffer<T> {
    pub(crate) fn new(vec: Vec<T>) -> Self {
        Self { buf: vec.into() }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn take_n<F: Fn(&[T])>(&mut self, _n: usize, _f: F) {
        todo!()
    }
}
