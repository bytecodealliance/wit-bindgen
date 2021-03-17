use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::mem;
use std::rc::Rc;
use std::slice;
use wasmtime::Trap;

#[derive(Default, Clone)]
pub struct BufferGlue {
    inner: Rc<RefCell<Inner>>,
}

#[derive(Default)]
struct Inner {
    in_buffers: Slab,
    out_buffers: Slab,
}

impl BufferGlue {
    /// Unsafe: must not leak the return value
    pub unsafe fn push_in_buffer<'a>(
        &'a self,
        buffer: &'a [u8],
        element_size: usize,
    ) -> InBufferHandle<'_> {
        let mut inner = self.inner.borrow_mut();
        let handle = inner.in_buffers.insert(Buffer {
            ptr: buffer.as_ptr() as usize,
            len: buffer.len(),
            element_size,
        });
        InBufferHandle {
            glue: self,
            _buffer: buffer,
            handle,
        }
    }

    /// Unsafe: must not leak the return value
    pub unsafe fn push_out_buffer<'a>(
        &'a self,
        buffer: &'a mut [u8],
        element_size: usize,
    ) -> OutBufferHandle<'_> {
        let mut inner = self.inner.borrow_mut();
        let handle = inner.out_buffers.insert(Buffer {
            ptr: buffer.as_ptr() as usize,
            len: buffer.len(),
            element_size,
        });
        OutBufferHandle {
            glue: self,
            _buffer: buffer,
            handle,
        }
    }

    pub fn in_len(&self, handle: u32) -> Result<u32, Trap> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .in_buffers
            .get_mut(handle)
            .ok_or_else(|| Trap::new("invalid in-buffer handle"))?;
        Ok((b.len / b.element_size).try_into().unwrap())
    }

    /// Implementation of the canonical abi "in_read" function
    pub fn in_read(
        &self,
        handle: u32,
        memory: &wasmtime::Memory,
        base: u32,
        len: u32,
    ) -> Result<(), Trap> {
        let mut inner = self.inner.borrow_mut();
        // Validate `handle` to make sure that it's valid...
        let b = inner
            .in_buffers
            .get_mut(handle)
            .ok_or_else(|| Trap::new("invalid in-buffer handle"))?;

        // Note the unsafety here. This should in theory be valid because a
        // `Handle` still exist somewhere else in the system which is holding
        // the buffer alive. This does rely on `Handle` not being leaked, but
        // that's part of the unsafety of the `push_*` functions.
        let src = unsafe { slice::from_raw_parts(b.ptr as *const u8, b.len) };

        // Calculate the byte length of the copy that's going to be performed.
        // This computation can overflow so we need to guard against that.
        let byte_len = usize::try_from(len)
            .unwrap()
            .checked_mul(b.element_size)
            .ok_or_else(|| Trap::new("overflow in requested size"))?;

        // Validate that we have enough bytes to satisfy the byte length
        // request. If not then the wasm module requested too many bytes and
        // that's a trap.
        let to_write = src
            .get(..byte_len)
            .ok_or_else(|| Trap::new("more bytes requested than are available to read"))?;

        // And lastly we actually attempt to copy the memory from the host (us)
        // to wasm by using `memory.write`. This can fail if the `base` pointer
        // is out of bounds or otherwise isn't valid.
        memory
            .write(usize::try_from(base).unwrap(), to_write)
            .map_err(|e| Trap::new(format!("invalid write into wasm memory: {}", e)))?;

        // And if we got this far then we finished! Update our ptr/len to
        // account for the number of bytes consumed.
        b.ptr += byte_len;
        b.len -= byte_len;
        Ok(())
    }

    pub fn out_len(&self, handle: u32) -> Result<u32, Trap> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .out_buffers
            .get_mut(handle)
            .ok_or_else(|| Trap::new("invalid out-buffer handle"))?;
        Ok((b.len / b.element_size).try_into().unwrap())
    }

    /// Implementation of the canonical abi "out_write" function
    pub fn out_write(
        &self,
        handle: u32,
        memory: &wasmtime::Memory,
        base: u32,
        len: u32,
    ) -> Result<(), Trap> {
        // The body of this function is quite similar to the above function.
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .out_buffers
            .get_mut(handle)
            .ok_or_else(|| Trap::new("invalid out-buffer handle"))?;
        let dst = unsafe { slice::from_raw_parts_mut(b.ptr as *mut u8, b.len) };
        let byte_len = usize::try_from(len)
            .unwrap()
            .checked_mul(b.element_size)
            .ok_or_else(|| Trap::new("overflow in requested size"))?;
        let dst = dst
            .get_mut(..byte_len)
            .ok_or_else(|| Trap::new("more bytes requested than are available to write"))?;
        memory
            .read(usize::try_from(base).unwrap(), dst)
            .map_err(|e| Trap::new(format!("invalid read from wasm memory: {}", e)))?;

        // And if we got this far then we finished! Update our ptr/len to
        // account for the number of bytes consumed.
        b.ptr += byte_len;
        b.len -= byte_len;
        Ok(())
    }
}

pub struct InBufferHandle<'a> {
    glue: &'a BufferGlue,
    _buffer: &'a [u8],
    handle: u32,
}

impl InBufferHandle<'_> {
    pub fn handle(&self) -> u32 {
        self.handle
    }
}

impl Drop for InBufferHandle<'_> {
    fn drop(&mut self) {
        let mut inner = self.glue.inner.borrow_mut();
        inner.in_buffers.remove(self.handle);
    }
}

pub struct OutBufferHandle<'a> {
    glue: &'a BufferGlue,
    _buffer: &'a mut [u8],
    handle: u32,
}

impl OutBufferHandle<'_> {
    pub fn handle(&self) -> u32 {
        self.handle
    }
}

impl Drop for OutBufferHandle<'_> {
    fn drop(&mut self) {
        let mut inner = self.glue.inner.borrow_mut();
        inner.out_buffers.remove(self.handle);
    }
}

#[derive(Default)]
struct Slab {
    storage: Vec<Entry>,
    next: usize,
}

enum Entry {
    Buffer(Buffer),
    Empty { next: usize },
}

struct Buffer {
    ptr: usize,
    len: usize,
    element_size: usize,
}

impl Slab {
    fn insert(&mut self, buffer: Buffer) -> u32 {
        if self.next == self.storage.len() {
            self.storage.push(Entry::Empty {
                next: self.next + 1,
            });
        }
        let ret = self.next as u32;
        let entry = Entry::Buffer(buffer);
        self.next = match mem::replace(&mut self.storage[self.next], entry) {
            Entry::Empty { next } => next,
            _ => unreachable!(),
        };
        return ret;
    }

    fn get_mut(&mut self, idx: u32) -> Option<&mut Buffer> {
        match self.storage.get_mut(idx as usize)? {
            Entry::Buffer(b) => Some(b),
            Entry::Empty { .. } => None,
        }
    }

    fn remove(&mut self, idx: u32) {
        self.storage[idx as usize] = Entry::Empty { next: self.next };
        self.next = idx as usize;
    }
}

/// Implementation of `(in-buffer T)`.
///
/// Holds a region of memory to store into as well as an iterator of items to
/// serialize when calling an API.
pub struct InBuffer<'a, T> {
    storage: &'a mut [u8],
    items: &'a mut dyn Iterator<Item = T>,
}

impl<'a, T: 'a> InBuffer<'a, T> {
    /// Creates a new buffer where `items` are serialized into `storage` when
    /// this buffer is passed to a function call.
    ///
    /// # Panics
    ///
    /// `storage` must be large enough to store all the `items`
    /// provided. This will panic otherwise when passed to a callee.
    pub fn new(storage: &'a mut [u8], items: &'a mut dyn Iterator<Item = T>) -> InBuffer<'a, T> {
        InBuffer { storage, items }
    }

    /// Called from adapters with implementation of how to serialize.
    #[doc(hidden)]
    pub fn serialize<F, const N: usize>(&mut self, mut write: F) -> Result<&[u8], Trap>
    where
        F: FnMut(&mut [u8], T) -> Result<(), Trap>,
    {
        let mut len = 0;
        for (i, item) in self.items.enumerate() {
            let storage = &mut self.storage[N * i..][..N];
            write(storage, item)?;
            len += 1;
        }
        Ok(&self.storage[..len * N])
    }
}

impl<T> fmt::Debug for InBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InBuffer")
            .field("bytes", &self.storage.len())
            .finish()
    }
}

/// Implementation of `(out-buffer T)`
pub struct OutBuffer<'a, T: 'a> {
    storage: &'a mut [u8],
    deserialize: fn(&[u8]) -> T,
    element_size: usize,
}

impl<'a, T: 'a> OutBuffer<'a, T> {
    /// Creates a new buffer with `storage` as where to store raw byte given to
    /// the host from wasm.
    ///
    /// The `storage` should be appropriately sized to hold the desired number
    /// of items to receive.
    pub fn new(storage: &'a mut [u8]) -> OutBuffer<'a, T> {
        OutBuffer {
            storage,
            deserialize: |_| loop {},
            element_size: usize::max_value(),
        }
    }

    #[doc(hidden)]
    pub fn storage(&mut self) -> &mut [u8] {
        &mut *self.storage
    }

    ///// Called from adapters with implementation of how to deserialize.
    //#[doc(hidden)]
    //pub fn ptr_len<const N: usize>(&mut self, deserialize: fn(i32) -> T) -> (i32, i32) {
    //    self.element_size = N;
    //    self.deserialize = deserialize;
    //    (
    //        self.storage.as_ptr() as i32,
    //        (self.storage.len() / N) as i32,
    //    )
    //}

    ///// Consumes this output buffer, returning an iterator of the deserialized
    ///// version of all items that callee wrote.
    /////
    ///// This is `unsafe` because the `amt` here is not known to be valid, and
    ///// deserializing arbitrary bytes is not safe. The callee should always
    ///// indicate how many items were written into this output buffer by some
    ///// other means.
    //pub unsafe fn into_iter(self, amt: usize) -> impl Iterator<Item = T> + 'a
    //where
    //    T: 'a,
    //{
    //    let size = self.element_size;
    //    (0..amt)
    //        .map(move |i| i * size)
    //        .map(move |i| (self.deserialize)(self.storage[i..][..size].as_mut_ptr() as i32))
    //    // TODO: data is leaked if this iterator isn't run in full
    //}
}

impl<T> fmt::Debug for OutBuffer<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutBuffer")
            .field("bytes", &self.storage.len())
            .finish()
    }
}
