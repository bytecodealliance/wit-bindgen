use std::fmt;

/// Implementation of `(in-buffer T)`.
///
/// Holds a region of memory to store into as well as an iterator of items to
/// serialize when calling an imported API.
pub struct PullBuffer<'a, T: 'a> {
    storage: &'a mut [u8],
    items: &'a mut dyn ExactSizeIterator<Item = T>,
    len: usize,
}

impl<'a, T: 'a> PullBuffer<'a, T> {
    /// Creates a new buffer where `items` are serialized into `storage` when
    /// this buffer is passed to a function call.
    ///
    /// Note that `storage` must be large enough to store all the `items`
    /// provided. This will panic otherwise when passed to a callee.
    pub fn new(
        storage: &'a mut [u8],
        items: &'a mut dyn ExactSizeIterator<Item = T>,
    ) -> PullBuffer<'a, T> {
        PullBuffer {
            len: items.len(),
            storage,
            items,
        }
    }

    /// Called from adapters with implementation of how to serialize.
    #[doc(hidden)]
    pub unsafe fn serialize<F, const N: usize>(&mut self, mut write: F) -> (i32, i32)
    where
        F: FnMut(T, i32),
    {
        for (i, item) in self.items.enumerate() {
            let storage = &mut self.storage[N * i..][..N];
            write(item, storage.as_mut_ptr() as i32);
        }
        (self.storage.as_ptr() as i32, self.len as i32)
    }
}

impl<T> fmt::Debug for PullBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PullBuffer")
            .field("bytes", &self.storage.len())
            .field("items", &self.items.len())
            .finish()
    }
}

/// Implementation of `(out-buffer T)`
pub struct PushBuffer<'a, T: 'a> {
    storage: &'a mut [u8],
    deserialize: fn(i32) -> T,
    element_size: usize,
}

impl<'a, T: 'a> PushBuffer<'a, T> {
    pub fn new(storage: &'a mut [u8]) -> PushBuffer<'a, T> {
        PushBuffer {
            storage,
            deserialize: |_| loop {},
            element_size: usize::max_value(),
        }
    }

    /// Called from adapters with implementation of how to deserialize.
    #[doc(hidden)]
    pub fn ptr_len<const N: usize>(&mut self, deserialize: fn(i32) -> T) -> (i32, i32) {
        self.element_size = N;
        self.deserialize = deserialize;
        (
            self.storage.as_ptr() as i32,
            (self.storage.len() / N) as i32,
        )
    }

    /// Consumes this output buffer, returning an iterator of the deserialized
    /// version of all items that callee wrote.
    ///
    /// This is `unsafe` because the `amt` here is not known to be valid, and
    /// deserializing arbitrary bytes is not safe. The callee should always
    /// indicate how many items were written into this output buffer by some
    /// other means.
    pub unsafe fn into_iter(self, amt: usize) -> impl Iterator<Item = T> + 'a
    where
        T: 'a,
    {
        let size = self.element_size;
        (0..amt)
            .map(move |i| i * size)
            .map(move |i| (self.deserialize)(self.storage[i..][..size].as_mut_ptr() as i32))
        // TODO: data is leaked if this iterator isn't run in full
    }
}

impl<T> fmt::Debug for PushBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PushBuffer")
            .field("bytes", &self.storage.len())
            .finish()
    }
}
