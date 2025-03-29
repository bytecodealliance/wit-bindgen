use crate::async_support::StreamVtable;
use crate::Cleanup;
use std::alloc::Layout;
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::vec::Vec;

/// A helper structure used with a stream to handle the canonical ABI
/// representation of lists and track partial writes.
///
/// This structure is returned whenever a write to a stream completes. This
/// keeps track of the original buffer used to perform a write (`Vec<T>`) and
/// additionally tracks any partial writes. Writes can then be resumed with
/// this buffer again or the partial write can be converted back to `Vec<T>` to
/// get access to the remaining values.
///
/// This value is created through the [`StreamWrite`](super::StreamWrite)
/// future's return value.
pub struct AbiBuffer<T: 'static> {
    rust_storage: Vec<MaybeUninit<T>>,
    vtable: &'static StreamVtable<T>,
    alloc: Option<Cleanup>,
    cursor: usize,
}

impl<T: 'static> AbiBuffer<T> {
    pub(crate) fn new(mut vec: Vec<T>, vtable: &'static StreamVtable<T>) -> AbiBuffer<T> {
        assert_eq!(vtable.lower.is_some(), vtable.lift.is_some());

        // SAFETY: We're converting `Vec<T>` to `Vec<MaybeUninit<T>>`, which
        // should be safe.
        let rust_storage = unsafe {
            let ptr = vec.as_mut_ptr();
            let len = vec.len();
            let cap = vec.capacity();
            mem::forget(vec);
            Vec::<MaybeUninit<T>>::from_raw_parts(ptr.cast(), len, cap)
        };

        // If `lower` is provided then the canonical ABI format is different
        // from the native format, so all items are converted at this time.
        //
        // Note that this is probably pretty inefficient for "big" use cases
        // but it's hoped that "big" use cases are using `u8` and therefore
        // skip this entirely.
        let alloc = vtable.lower.and_then(|lower| {
            let layout = Layout::from_size_align(
                vtable.layout.size() * rust_storage.len(),
                vtable.layout.align(),
            )
            .unwrap();
            let (mut ptr, cleanup) = Cleanup::new(layout);
            let cleanup = cleanup?;
            // SAFETY: All items in `rust_storage` are already initialized so
            // it should be safe to read them and move ownership into the
            // canonical ABI format.
            unsafe {
                for item in rust_storage.iter() {
                    let item = item.assume_init_read();
                    lower(item, ptr);
                    ptr = ptr.add(vtable.layout.size());
                }
            }

            Some(cleanup)
        });
        AbiBuffer {
            rust_storage,
            alloc,
            vtable,
            cursor: 0,
        }
    }

    /// Returns the canonical ABI pointer/length to pass off to a write
    /// operation.
    pub(crate) fn abi_ptr_and_len(&self) -> (*const u8, usize) {
        // If there's no `lower` operation then it menas that `T`'s layout is
        // the same in the canonical ABI so it can be used as-is. In this
        // situation the list would have been un-tampered with above.
        if self.vtable.lower.is_none() {
            // SAFETY: this should be in-bounds, so it should be safe.
            let ptr = unsafe { self.rust_storage.as_ptr().add(self.cursor).cast() };
            let len = self.rust_storage.len() - self.cursor;
            return (ptr, len.try_into().unwrap());
        }

        // Othereise when `lower` is present that means that `self.alloc` has
        // the ABI pointer we should pass along.
        let ptr = self
            .alloc
            .as_ref()
            .map(|c| c.ptr.as_ptr())
            .unwrap_or(ptr::null_mut());
        (
            // SAFETY: this should be in-bounds, so it should be safe.
            unsafe { ptr.add(self.cursor * self.vtable.layout.size()) },
            self.rust_storage.len() - self.cursor,
        )
    }

    /// Converts this `AbiBuffer<T>` back into a `Vec<T>`
    ///
    /// This commit consumes this buffer and yields back unwritten values as a
    /// `Vec<T>`. The remaining items in `Vec<T>` have not yet been written and
    /// all written items have been removed from the front of the list.
    ///
    /// Note that the backing storage of the returned `Vec<T>` has not changed
    /// from whe this buffer was created.
    ///
    /// Also note that this can be an expensive operation if a partial write
    /// occurred as this will involve shifting items from the end of the vector
    /// to the start of the vector.
    pub fn into_vec(mut self) -> Vec<T> {
        self.take_vec()
    }

    /// Returns the number of items remaining in this buffer.
    pub fn remaining(&self) -> usize {
        self.rust_storage.len() - self.cursor
    }

    /// Advances this buffer by `amt` items.
    ///
    /// This signals that `amt` items are no longer going to be yielded from
    /// `abi_ptr_and_len`. Additionally this will perform any deallocation
    /// necessary for the starting `amt` items in this list.
    pub(crate) fn advance(&mut self, amt: usize) {
        assert!(amt + self.cursor <= self.rust_storage.len());
        let Some(dealloc_lists) = self.vtable.dealloc_lists else {
            self.cursor += amt;
            return;
        };
        let (mut ptr, len) = self.abi_ptr_and_len();
        assert!(amt <= len);
        for _ in 0..amt {
            // SAFETY: we're managing the pointer passed to `dealloc_lists` and
            // it was initialized with a `lower`, and then the pointer
            // arithmetic should all be in-bounds.
            unsafe {
                dealloc_lists(ptr.cast_mut());
                ptr = ptr.add(self.vtable.layout.size());
            }
        }
        self.cursor += amt;
    }

    fn take_vec(&mut self) -> Vec<T> {
        // First, if necessary, convert remaining values within `self.alloc`
        // back into `self.rust_storage`. This is necessary when a lift
        // operation is available meaning that the representation of `T` is
        // different in the canonical ABI.
        //
        // Note that when `lift` is provided then when this original
        // `AbiBuffer` was created it moved ownership of all values from the
        // original vector into the `alloc` value. This is the reverse
        // operation, moving all the values back into the vector.
        if let Some(lift) = self.vtable.lift {
            let (mut ptr, mut len) = self.abi_ptr_and_len();
            // SAFETY: this should be safe as `lift` is operating on values that
            // were initialized with a previous `lower`, and the pointer
            // arithmetic here should all be in-bounds.
            unsafe {
                for dst in self.rust_storage[self.cursor..].iter_mut() {
                    dst.write(lift(ptr.cast_mut()));
                    ptr = ptr.add(self.vtable.layout.size());
                    len -= 1;
                }
                assert_eq!(len, 0);
            }
        }

        // Next extract the rust storage and zero out this struct's fields.
        // This is also the location where a "shift" happens to remove items
        // from the beginning of the returned vector as those have already been
        // transferred somewhere else.
        let mut storage = mem::take(&mut self.rust_storage);
        storage.drain(..self.cursor);
        self.cursor = 0;
        self.alloc = None;

        // SAFETY: we're casting `Vec<MaybeUninit<T>>` here to `Vec<T>`. The
        // elements were either always initialized (`lift` is `None`) or we just
        // re-initialized them above from `self.alloc`.
        unsafe {
            let ptr = storage.as_mut_ptr();
            let len = storage.len();
            let cap = storage.capacity();
            mem::forget(storage);
            Vec::<T>::from_raw_parts(ptr.cast(), len, cap)
        }
    }
}

impl<T> Drop for AbiBuffer<T> {
    fn drop(&mut self) {
        let _ = self.take_vec();
    }
}
