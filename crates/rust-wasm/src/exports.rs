use std::fmt;
use std::marker;

#[link(wasm_import_module = "witx_canonical_buffer_abi")]
extern "C" {
    fn in_len(handle: i32) -> usize;
    fn in_read(handle: i32, amount: usize, dst: *mut u8);
    fn out_len(handle: i32) -> usize;
    fn out_write(handle: i32, amount: usize, dst: *const u8);
}

/// Implementation of `(in-buffer T)` for raw types `T` that can be directly
/// copied into.
pub struct InBufferRaw<'a, T> {
    handle: i32,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T> InBufferRaw<'a, T> {
    /// Only intended for adapter use.
    ///
    /// `unsafe` because this requires a valid `handle` and also requires `T` to
    /// be valid to copy into. Additionally requires a valid `'a`
    pub unsafe fn new(handle: i32) -> InBufferRaw<'a, T> {
        InBufferRaw {
            handle,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the length of the buffer provided by the caller.
    ///
    /// Returns the number of items, in units of `T`, that are available to
    /// `copy` to receive.
    pub fn len(&self) -> usize {
        unsafe { in_len(self.handle) }
    }

    /// Copies elements from the caller into `space`.
    ///
    /// This will abort the program if `space` is larger than `self.len()`.
    /// Otherwise the `space` array will be entirely filled upon returning.
    pub fn copy(&self, space: &mut [T]) {
        unsafe {
            in_read(self.handle, space.len(), space.as_mut_ptr() as *mut u8);
        }
    }
}

impl<T> fmt::Debug for InBufferRaw<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InBufferRaw")
            .field("handle", &self.handle)
            .field("len", &self.len())
            .finish()
    }
}

/// Implementation of `(in-buffer T)`
pub struct InBuffer<'a, T> {
    handle: i32,
    deserialize: fn(i32) -> T,
    element_size: usize,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T> InBuffer<'a, T> {
    /// Only intended for adapter use.
    pub unsafe fn new(
        handle: i32,
        element_size: i32,
        deserialize: fn(i32) -> T,
    ) -> InBuffer<'a, T> {
        InBuffer {
            handle,
            element_size: element_size as u32 as usize,
            deserialize,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the length of the buffer provided by the caller.
    ///
    /// Returns the number of items, in units of `T`, that are available to
    /// `iter` to copy in.
    pub fn len(&self) -> usize {
        unsafe { in_len(self.handle) }
    }

    /// Returns the size of a `T` to gauge how much scratch space to pass to
    /// [`InBuffer::iter`].
    pub fn element_size(&self) -> usize {
        self.element_size
    }

    /// Copies items from the caller into `scratch` and then returns an
    /// iterator over the deserialized versions.
    ///
    /// The `scratch` buffer should be appropriately sized for the number of
    /// items you wish to iterate over.
    pub fn iter<'b>(&self, scratch: &'b mut [u8]) -> impl ExactSizeIterator<Item = T> + 'b
    where
        'a: 'b,
    {
        // TODO: need to deserialize/drop remaining items if the iterator
        // doesn't finish
        unsafe {
            let element_size = self.element_size;
            let len = scratch.len() / element_size;
            in_read(self.handle, len, scratch.as_mut_ptr());
            let deserialize = self.deserialize;
            (0..len).map(move |i| {
                deserialize(scratch[i * element_size..][..element_size].as_ptr() as i32)
            })
        }
    }
}

impl<T> fmt::Debug for InBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InBuffer")
            .field("handle", &self.handle)
            .field("len", &self.len())
            .finish()
    }
}

/// Implementation of `(out-buffer T)` for raw types `T` that can be directly
/// copied to the caller.
pub struct OutBufferRaw<'a, T> {
    handle: i32,
    _marker: marker::PhantomData<&'a mut T>,
}

impl<'a, T> OutBufferRaw<'a, T> {
    /// Only intended for adapter use.
    ///
    /// `unsafe` because this requires a valid `handle`, requires `T` to
    /// be valid to copy into, and requires a valid `'a`.
    pub unsafe fn new(handle: i32) -> OutBufferRaw<'a, T> {
        OutBufferRaw {
            handle,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the capacity of the buffer provided by the caller.
    ///
    /// Returns the number of items, in units of `T`, that are available to
    /// `write` to receive.
    pub fn capacity(&self) -> usize {
        unsafe { out_len(self.handle) }
    }

    /// Copies elements to the caller from `items`.
    ///
    /// This will abort the program if `items` is larger than `self.capacity()`.
    pub fn write(&self, items: &[T]) {
        unsafe {
            out_write(self.handle, items.len(), items.as_ptr() as *const u8);
        }
    }
}

impl<T> fmt::Debug for OutBufferRaw<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutBufferRaw")
            .field("handle", &self.handle)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// Implementation of `(in-buffer T)`
pub struct OutBuffer<'a, T> {
    handle: i32,
    serialize: fn(i32, T),
    element_size: usize,
    _marker: marker::PhantomData<&'a mut T>,
}

impl<'a, T> OutBuffer<'a, T> {
    /// Only intended for adapter use.
    pub unsafe fn new(handle: i32, element_size: i32, serialize: fn(i32, T)) -> OutBuffer<'a, T> {
        OutBuffer {
            handle,
            element_size: element_size as u32 as usize,
            serialize,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the capacity of the buffer provided by the caller.
    ///
    /// Returns the number of items, in units of `T`, that are available to
    /// `iter` to copy in.
    pub fn capacity(&self) -> usize {
        unsafe { out_len(self.handle) }
    }

    /// Returns the size of a `T` to gauge how much scratch space to pass to
    /// [`OutBuffer::write`].
    pub fn element_size(&self) -> usize {
        self.element_size
    }

    /// Writes items into this buffer.
    ///
    /// This method will write the `items` provided into this buffer to get
    /// passed to the caller. The `scratch` space provided must be large enough
    /// to contain the encoded size of all of `items`, and the amount of
    /// `scratch` needed can be gauged with the [`OutBuffer::element_size`]
    /// method.
    pub fn write(&self, scratch: &mut [u8], items: impl ExactSizeIterator<Item = T>) {
        assert!(items.len().checked_mul(self.element_size).unwrap() <= scratch.len());
        let mut len = 0;
        // TODO: if `items` ends up being longer than expected then we leak all
        // items previously serialized.
        for (i, item) in items.enumerate() {
            len += 1;
            (self.serialize)(
                scratch[i * self.element_size..][..self.element_size].as_mut_ptr() as i32,
                item,
            );
        }
        unsafe {
            out_write(self.handle, len, scratch.as_ptr());
        }
    }
}

impl<T> fmt::Debug for OutBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutBuffer")
            .field("handle", &self.handle)
            .field("capacity", &self.capacity())
            .finish()
    }
}
