use crate::slab::Slab;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::mem;
use std::rc::Rc;
use wasmer::{Memory, RuntimeError};

#[derive(Default, Clone)]
pub struct BufferGlue {
    inner: Rc<RefCell<Inner>>,
}

#[derive(Default)]
struct Inner {
    in_buffers: Slab<Buffer<Input>>,
    out_buffers: Slab<Buffer<Output>>,
}

struct Buffer<T> {
    len: u32,
    kind: T,
}

enum Input {
    Bytes(*const u8, usize),
    General {
        shim: unsafe fn(
            [usize; 2],
            *const u8,
            &Memory,
            i32,
            u32,
            &mut u32,
        ) -> Result<(), RuntimeError>,
        iterator: [usize; 2],
        serialize: *const u8,
    },
}

enum Output {
    Bytes(*mut u8, usize),
    General {
        shim: unsafe fn(*mut u8, *const u8, &Memory, i32, u32) -> Result<(), RuntimeError>,
        dst: *mut u8,
        deserialize: *const u8,
    },
}

impl BufferGlue {
    pub fn transaction(&self) -> BufferTransaction<'_> {
        BufferTransaction {
            handles: Vec::new(),
            glue: self,
        }
    }

    pub fn in_len(&self, handle: u32) -> Result<u32, RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .in_buffers
            .get_mut(handle)
            .ok_or_else(|| RuntimeError::new("invalid in-buffer handle"))?;
        Ok(b.len)
    }

    /// Implementation of the canonical abi "in_read" function
    pub fn in_read(
        &self,
        handle: u32,
        memory: &wasmer::Memory,
        base: u32,
        len: u32,
    ) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .in_buffers
            .get_mut(handle)
            .ok_or_else(|| RuntimeError::new("invalid in-buffer handle"))?;
        if len > b.len {
            return Err(RuntimeError::new(
                "more items requested from in-buffer than are available",
            ));
        }
        unsafe {
            match &mut b.kind {
                Input::Bytes(ptr, elem_size) => {
                    let write_size = (len as usize) * *elem_size;
                    let dest = memory
                        .data_unchecked_mut()
                        .get_mut(base as usize..base as usize + write_size)
                        .ok_or_else(|| {
                            RuntimeError::new("out-of-bounds write while reading in-buffer")
                        })?;
                    dest.copy_from_slice(std::slice::from_raw_parts(*ptr, write_size));
                    *ptr = (*ptr).add(write_size);
                    b.len -= len;
                    Ok(())
                }
                &mut Input::General {
                    shim,
                    iterator,
                    serialize,
                } => {
                    drop(inner);
                    let mut processed = 0;
                    let res = shim(
                        iterator,
                        serialize,
                        memory,
                        base as i32,
                        len,
                        &mut processed,
                    );
                    self.inner
                        .borrow_mut()
                        .in_buffers
                        .get_mut(handle)
                        .expect("should still be there")
                        .len -= processed;
                    res
                }
            }
        }
    }

    pub fn out_len(&self, handle: u32) -> Result<u32, RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .out_buffers
            .get_mut(handle)
            .ok_or_else(|| RuntimeError::new("out in-buffer handle"))?;
        Ok(b.len)
    }

    /// Implementation of the canonical abi "out_write" function
    pub fn out_write(
        &self,
        handle: u32,
        memory: &wasmer::Memory,
        base: u32,
        len: u32,
    ) -> Result<(), RuntimeError> {
        let mut inner = self.inner.borrow_mut();
        let b = inner
            .out_buffers
            .get_mut(handle)
            .ok_or_else(|| RuntimeError::new("invalid out-buffer handle"))?;
        if len > b.len {
            return Err(RuntimeError::new(
                "more items written to out-buffer than are available",
            ));
        }
        unsafe {
            match &mut b.kind {
                Output::Bytes(ptr, elem_size) => {
                    let read_size = (len as usize) * *elem_size;
                    let src = memory
                        .data_unchecked()
                        .get(base as usize..base as usize + read_size)
                        .ok_or_else(|| {
                            RuntimeError::new("out-of-bounds read while writing to out-buffer")
                        })?;
                    std::slice::from_raw_parts_mut(*ptr, read_size).copy_from_slice(src);
                    *ptr = (*ptr).add(read_size);
                    b.len -= len;
                    Ok(())
                }
                &mut Output::General {
                    shim,
                    dst,
                    deserialize,
                } => {
                    shim(dst, deserialize, memory, base as i32, len)?;
                    b.len -= len;
                    Ok(())
                }
            }
        }
    }
}

pub struct BufferTransaction<'a> {
    glue: &'a BufferGlue,
    handles: Vec<(bool, u32)>,
}

impl<'call> BufferTransaction<'call> {
    pub unsafe fn push_in_raw<'a, T>(&mut self, buffer: &'a [T]) -> i32
    where
        'a: 'call,
    {
        let mut inner = self.glue.inner.borrow_mut();
        let handle = inner.in_buffers.insert(Buffer {
            len: u32::try_from(buffer.len()).unwrap(),
            kind: Input::Bytes(buffer.as_ptr() as *const u8, mem::size_of::<T>()),
        });
        self.handles.push((false, handle));
        return handle as i32;
    }

    pub unsafe fn push_in<'a, T, F>(
        &mut self,
        iter: &'a mut (dyn ExactSizeIterator<Item = T> + 'a),
        write: &'a F,
    ) -> i32
    where
        F: Fn(&Memory, i32, T) -> Result<i32, RuntimeError> + 'a,
        'a: 'call,
    {
        let mut inner = self.glue.inner.borrow_mut();
        let handle = inner.in_buffers.insert(Buffer {
            len: u32::try_from(iter.len()).unwrap(),
            kind: Input::General {
                shim: shim::<T, F>,
                iterator: mem::transmute(iter),
                serialize: write as *const F as *const u8,
            },
        });
        self.handles.push((false, handle));
        return handle as i32;

        unsafe fn shim<T, F>(
            iter: [usize; 2],
            serialize: *const u8,
            memory: &Memory,
            mut offset: i32,
            len: u32,
            processed: &mut u32,
        ) -> Result<(), RuntimeError>
        where
            F: Fn(&Memory, i32, T) -> Result<i32, RuntimeError>,
        {
            let iter = mem::transmute::<_, &mut dyn ExactSizeIterator<Item = T>>(iter);
            let write = &*(serialize as *const F);
            for _ in 0..len {
                let item = iter.next().unwrap();
                offset += write(memory, offset, item)?;
                *processed += 1;
            }
            Ok(())
        }
    }

    pub unsafe fn push_out_raw<'a, T>(&mut self, buffer: &'a mut [T]) -> i32
    where
        'a: 'call,
    {
        let mut inner = self.glue.inner.borrow_mut();
        let handle = inner.out_buffers.insert(Buffer {
            len: u32::try_from(buffer.len()).unwrap(),
            kind: Output::Bytes(buffer.as_mut_ptr() as *mut u8, mem::size_of::<T>()),
        });
        self.handles.push((true, handle));
        return handle as i32;
    }

    pub unsafe fn push_out<'a, T, F>(&mut self, dst: &'a mut Vec<T>, read: &'a F) -> i32
    where
        F: Fn(&Memory, i32) -> Result<(T, i32), RuntimeError> + 'a,
        'a: 'call,
    {
        let mut inner = self.glue.inner.borrow_mut();
        let handle = inner.out_buffers.insert(Buffer {
            len: u32::try_from(dst.capacity() - dst.len()).unwrap(),
            kind: Output::General {
                shim: shim::<T, F>,
                dst: dst as *mut Vec<T> as *mut u8,
                deserialize: read as *const F as *const u8,
            },
        });
        self.handles.push((true, handle));
        return handle as i32;

        unsafe fn shim<T, F>(
            dst: *mut u8,
            deserialize: *const u8,
            memory: &Memory,
            mut offset: i32,
            len: u32,
        ) -> Result<(), RuntimeError>
        where
            F: Fn(&Memory, i32) -> Result<(T, i32), RuntimeError>,
        {
            let dst = &mut *(dst as *mut Vec<T>);
            let read = &*(deserialize as *const F);
            for _ in 0..len {
                let (item, size) = read(memory, offset)?;
                dst.push(item);
                offset += size;
            }
            Ok(())
        }
    }
}

impl Drop for BufferTransaction<'_> {
    fn drop(&mut self) {
        let mut inner = self.glue.inner.borrow_mut();
        for (out, handle) in self.handles.iter() {
            if *out {
                inner.out_buffers.remove(*handle);
            } else {
                inner.in_buffers.remove(*handle);
            }
        }
    }
}

///// Implementation of `(in-buffer T)`.
/////
///// Holds a region of memory to store into as well as an iterator of items to
///// serialize when calling an API.
//pub struct InBuffer<'a, T> {
//    storage: &'a mut [u8],
//    items: &'a mut dyn Iterator<Item = T>,
//}

//impl<'a, T: 'a> InBuffer<'a, T> {
//    /// Creates a new buffer where `items` are serialized into `storage` when
//    /// this buffer is passed to a function call.
//    ///
//    /// # Panics
//    ///
//    /// `storage` must be large enough to store all the `items`
//    /// provided. This will panic otherwise when passed to a callee.
//    pub fn new(storage: &'a mut [u8], items: &'a mut dyn Iterator<Item = T>) -> InBuffer<'a, T> {
//        InBuffer { storage, items }
//    }

//    /// Called from adapters with implementation of how to serialize.
//    #[doc(hidden)]
//    pub fn serialize<F, const N: usize>(&mut self, mut write: F) -> Result<&[u8], RuntimeError>
//    where
//        F: FnMut(&mut [u8], T) -> Result<(), RuntimeError>,
//    {
//        let mut len = 0;
//        for (i, item) in self.items.enumerate() {
//            let storage = &mut self.storage[N * i..][..N];
//            write(storage, item)?;
//            len += 1;
//        }
//        Ok(&self.storage[..len * N])
//    }
//}

//impl<T> fmt::Debug for InBuffer<'_, T> {
//    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        f.debug_struct("InBuffer")
//            .field("bytes", &self.storage.len())
//            .finish()
//    }
//}

///// Implementation of `(out-buffer T)`
//pub struct OutBuffer<'a, T: 'a> {
//    storage: &'a mut [u8],
//    deserialize: fn(&[u8]) -> T,
//    element_size: usize,
//}

//impl<'a, T: 'a> OutBuffer<'a, T> {
//    /// Creates a new buffer with `storage` as where to store raw byte given to
//    /// the host from wasm.
//    ///
//    /// The `storage` should be appropriately sized to hold the desired number
//    /// of items to receive.
//    pub fn new(storage: &'a mut [u8]) -> OutBuffer<'a, T> {
//        OutBuffer {
//            storage,
//            deserialize: |_| loop {},
//            element_size: usize::max_value(),
//        }
//    }

//    #[doc(hidden)]
//    pub fn storage(
//        &mut self,
//        _: usize,
//        _: impl Fn(&[u8]) -> Result<T, RuntimeError> + Clone + 'static,
//    ) -> &mut [u8] {
//        &mut *self.storage
//    }

//    ///// Called from adapters with implementation of how to deserialize.
//    //#[doc(hidden)]
//    //pub fn ptr_len<const N: usize>(&mut self, deserialize: fn(i32) -> T) -> (i32, i32) {
//    //    self.element_size = N;
//    //    self.deserialize = deserialize;
//    //    (
//    //        self.storage.as_ptr() as i32,
//    //        (self.storage.len() / N) as i32,
//    //    )
//    //}

//    ///// Consumes this output buffer, returning an iterator of the deserialized
//    ///// version of all items that callee wrote.
//    /////
//    ///// This is `unsafe` because the `amt` here is not known to be valid, and
//    ///// deserializing arbitrary bytes is not safe. The callee should always
//    ///// indicate how many items were written into this output buffer by some
//    ///// other means.
//    //pub unsafe fn into_iter(self, amt: usize) -> impl Iterator<Item = T> + 'a
//    //where
//    //    T: 'a,
//    //{
//    //    let size = self.element_size;
//    //    (0..amt)
//    //        .map(move |i| i * size)
//    //        .map(move |i| (self.deserialize)(self.storage[i..][..size].as_mut_ptr() as i32))
//    //    // TODO: data is leaked if this iterator isn't run in full
//    //}
//}

//impl<T> fmt::Debug for OutBuffer<'_, T> {
//    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        f.debug_struct("OutBuffer")
//            .field("bytes", &self.storage.len())
//            .finish()
//    }
//}
