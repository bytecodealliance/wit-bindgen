use std::fmt;
use std::marker;
use std::mem;
use std::ops::Deref;

#[cfg(feature = "macros")]
pub use wit_bindgen_rust_impl::{export, import};

#[cfg(feature = "async")]
pub use async_trait::async_trait;
#[cfg(feature = "async")]
mod futures;

// Re-export `bitflags` so that we can reference it from macros.
#[doc(hidden)]
pub use bitflags;

/// A type for handles to resources that appear in exported functions.
///
/// This type is used as `Handle<T>` for argument types and return values of
/// exported functions when exposing a Rust-defined resource. A `Handle<T>`
/// represents an owned reference count on the interface-types-managed resource.
/// It's similar to an `Rc<T>` in Rust. Internally a `Handle<T>` can provide
/// access to `&T` when `T` is defined in the current module.
pub struct Handle<T: HandleType> {
    val: i32,
    _marker: marker::PhantomData<T>,
}

impl<T: HandleType> Handle<T> {
    /// Creates a new handle around the specified value.
    ///
    /// Note that the lifetime of `T` will afterwards be managed by the
    /// interface types runtime. The host may hold references to `T` that wasm
    /// isn't necessarily aware of, preventing its destruction. Nevertheless
    /// though the `Drop for T` implementation will still be run when there are
    /// no more references to `T`.
    pub fn new(val: T) -> Handle<T>
    where
        T: LocalHandle,
    {
        unsafe { Handle::from_raw(T::new(Box::into_raw(Box::new(val)) as i32)) }
    }

    /// Consumes a handle and returns the underlying raw wasm i32 descriptor.
    ///
    /// Note that this, if used improperly, will leak the resource `T`. This
    /// generally should be avoided unless you're calling raw ABI bindings and
    /// managing the lifetime manually.
    pub fn into_raw(handle: Handle<T>) -> i32 {
        let ret = handle.val;
        mem::forget(handle);
        ret
    }

    /// Returns the raw underlying handle value for this handle.
    ///
    /// This typically isn't necessary to interact with, but can be useful when
    /// interacting with raw ABI bindings.
    pub fn as_raw(handle: &Handle<T>) -> i32 {
        handle.val
    }

    /// Unsafely assumes that the given integer descriptor is a handle for `T`.
    ///
    /// This is unsafe because no validation is performed to ensure that `val`
    /// is actually a valid descriptor for `T`.
    pub unsafe fn from_raw(val: i32) -> Handle<T> {
        Handle {
            val,
            _marker: marker::PhantomData,
        }
    }
}

impl<T: LocalHandle> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*(T::get(self.val) as *const T) }
    }
}

impl<T: LocalHandle> From<T> for Handle<T> {
    fn from(val: T) -> Handle<T> {
        Handle::new(val)
    }
}

impl<T: HandleType> Clone for Handle<T> {
    fn clone(&self) -> Self {
        unsafe { Handle::from_raw(T::clone(self.val)) }
    }
}

impl<T: HandleType> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle").field("val", &self.val).finish()
    }
}

impl<T: HandleType> Drop for Handle<T> {
    fn drop(&mut self) {
        T::drop(self.val);
    }
}

/// A trait for types that can show up as the `T` in `Handle<T>`.
///
/// This trait is automatically synthesized for exported handles and typically
/// shouldn't be implemented manually.
pub unsafe trait HandleType {
    #[doc(hidden)]
    fn clone(val: i32) -> i32;
    #[doc(hidden)]
    fn drop(val: i32);
}

/// An extension of the [`HandleType`] trait for locally-defined types.
///
/// This trait may not stick around forever...
pub unsafe trait LocalHandle: HandleType {
    #[doc(hidden)]
    fn new(val: i32) -> i32;
    #[doc(hidden)]
    fn get(val: i32) -> i32;
}

#[doc(hidden)]
pub mod rt {
    use std::alloc::{self, Layout};

    #[cfg(feature = "async")]
    pub use crate::futures::*;

    #[no_mangle]
    unsafe extern "C" fn canonical_abi_realloc(
        old_ptr: *mut u8,
        old_len: usize,
        align: usize,
        new_len: usize,
    ) -> *mut u8 {
        let layout;
        let ptr = if old_len == 0 {
            if new_len == 0 {
                return align as *mut u8;
            }
            layout = Layout::from_size_align_unchecked(new_len, align);
            alloc::alloc(layout)
        } else {
            layout = Layout::from_size_align_unchecked(old_len, align);
            alloc::realloc(old_ptr, layout, new_len)
        };
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        return ptr;
    }

    #[no_mangle]
    pub unsafe extern "C" fn canonical_abi_free(ptr: *mut u8, len: usize, align: usize) {
        if len == 0 {
            return;
        }
        let layout = Layout::from_size_align_unchecked(len, align);
        alloc::dealloc(ptr, layout);
    }

    macro_rules! as_traits {
        ($(($trait_:ident $func:ident $ty:ident <=> $($tys:ident)*))*) => ($(
            pub fn $func<T: $trait_>(t: T) -> $ty {
                t.$func()
            }

            pub trait $trait_ {
                fn $func(self) -> $ty;
            }

            impl<'a, T: Copy + $trait_> $trait_ for &'a T {
                fn $func(self) -> $ty{
                    (*self).$func()
                }
            }

            $(
                impl $trait_ for $tys {
                    #[inline]
                    fn $func(self) -> $ty {
                        self as $ty
                    }
                }
            )*

        )*)
    }

    as_traits! {
        (AsI64 as_i64 i64 <=> i64 u64)
        (AsI32 as_i32 i32 <=> i32 u32 i16 u16 i8 u8 char usize)
        (AsF32 as_f32 f32 <=> f32)
        (AsF64 as_f64 f64 <=> f64)
    }
}
