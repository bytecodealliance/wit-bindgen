use crate::{BorrowHandle, GuestError, GuestMemory, Region};
use std::cell::Cell;
use std::convert::TryFrom;
use std::fmt;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

pub struct GuestPtr<'a, T: ?Sized + Pointee> {
    mem: &'a (dyn GuestMemory + 'a),
    pointer: T::Pointer,
    _marker: marker::PhantomData<&'a Cell<T>>,
}

impl<'a, T: ?Sized + Pointee> GuestPtr<'a, T> {
    /// # Safety
    ///
    /// Only safe for types `T` which share the same in-memory representation in
    /// the guest and on the host. It's recommended to use a bindings generator
    /// to synthesize calls to `new` here, not call this manually.
    pub unsafe fn new(mem: &'a (dyn GuestMemory + 'a), pointer: T::Pointer) -> GuestPtr<'a, T> {
        GuestPtr {
            mem,
            pointer,
            _marker: marker::PhantomData,
        }
    }

    pub fn offset(&self) -> T::Pointer {
        self.pointer
    }

    pub fn mem(&self) -> &'a (dyn GuestMemory + 'a) {
        self.mem
    }

    pub fn borrow(&self) -> Result<Borrow<'a, T>, GuestError> {
        unsafe {
            let (raw, borrow) = T::validate(self.pointer, false, self.mem)?;
            Ok(Borrow {
                ptr: &*raw,
                mem: self.mem,
                borrow,
            })
        }
    }

    pub fn borrow_mut(&self) -> Result<BorrowMut<'a, T>, GuestError> {
        unsafe {
            let (raw, borrow) = T::validate(self.pointer, true, self.mem)?;
            Ok(BorrowMut {
                ptr: &mut *raw,
                mem: self.mem,
                borrow,
            })
        }
    }
}

impl<'a, T> GuestPtr<'a, [T]> {
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns a `GuestPtr` pointing to the base of the array for the interior
    /// type `T`.
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        // If `[T]` is a valid guest pointer then `T` is surely a valid guest
        // pointer type, hence wrapping the unsafety here.
        unsafe { GuestPtr::new(self.mem, self.offset_base()) }
    }
}

impl<'a> GuestPtr<'a, str> {
    /// For strings, returns the relative pointer to the base of the string
    /// allocation.
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// Returns the length, in bytes, of the string.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns a raw pointer for the underlying slice of bytes that this
    /// pointer points to.
    pub fn as_bytes(&self) -> GuestPtr<'a, [u8]> {
        // We know that `GuestPtr<[u8]>` is valid, hence the unsafety-wrapping
        // here.
        unsafe { GuestPtr::new(self.mem, self.pointer) }
    }
}

impl<T: ?Sized + Pointee> Clone for GuestPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee> Copy for GuestPtr<'_, T> {}

impl<T: ?Sized + Pointee> fmt::Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::debug(self.pointer, f)
    }
}

pub struct Borrow<'a, T: ?Sized> {
    ptr: &'a T,
    mem: &'a (dyn GuestMemory + 'a),
    borrow: BorrowHandle,
}

impl<'a, T: ?Sized> std::ops::Deref for Borrow<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a, T: ?Sized> Drop for Borrow<'a, T> {
    fn drop(&mut self) {
        self.mem.shared_unborrow(self.borrow)
    }
}

pub struct BorrowMut<'a, T: ?Sized> {
    ptr: &'a mut T,
    mem: &'a (dyn GuestMemory + 'a),
    borrow: BorrowHandle,
}

impl<'a, T: ?Sized> std::ops::Deref for BorrowMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a, T: ?Sized> std::ops::DerefMut for BorrowMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
    }
}

impl<'a, T: ?Sized> Drop for BorrowMut<'a, T> {
    fn drop(&mut self) {
        self.mem.mut_unborrow(self.borrow)
    }
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for T {}
    impl<T> Sealed for [T] {}
    impl Sealed for str {}
}

pub trait Pointee: private::Sealed {
    #[doc(hidden)]
    type Pointer: Copy;
    #[doc(hidden)]
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result;

    /// Validates that the pointer is safe for `Self` within the `GuestMemory`.
    ///
    /// The safety here is that the returned pointer is only valid so long as
    /// `GuestMemory`'s base doesn't change and no other modifications happen
    /// after this validity check is performed.
    #[doc(hidden)]
    unsafe fn validate(
        pointer: Self::Pointer,
        borrow_mut: bool,
        mem: &dyn GuestMemory,
    ) -> Result<(*mut Self, BorrowHandle), GuestError>;
}

impl<T> Pointee for T {
    type Pointer = u32;

    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}", pointer)
    }

    unsafe fn validate(
        pointer: Self::Pointer,
        borrow_mut: bool,
        mem: &dyn GuestMemory,
    ) -> Result<(*mut Self, BorrowHandle), GuestError> {
        let region = Region {
            start: pointer,
            len: u32::try_from(mem::size_of::<T>())?,
        };
        let ptr = mem.validate_size_align(region, mem::align_of::<T>())?;
        let borrow = if borrow_mut {
            mem.mut_borrow(region)?
        } else {
            mem.shared_borrow(region)?
        };
        Ok((ptr as *mut T, borrow))
    }
}

impl<T> Pointee for [T] {
    type Pointer = (u32, u32);

    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}/{}", pointer.0, pointer.1)
    }

    unsafe fn validate(
        (ptr, len): Self::Pointer,
        borrow_mut: bool,
        mem: &dyn GuestMemory,
    ) -> Result<(*mut Self, BorrowHandle), GuestError> {
        let region = Region {
            start: ptr,
            len: len
                .checked_mul(u32::try_from(mem::size_of::<T>())?)
                .ok_or(GuestError::PtrOverflow)?,
        };
        let ptr = mem.validate_size_align(region, mem::align_of::<T>())?;
        let usize_len = usize::try_from(len)?;
        let borrow = if borrow_mut {
            mem.mut_borrow(region)?
        } else {
            mem.shared_borrow(region)?
        };
        Ok((ptr::slice_from_raw_parts_mut(ptr.cast(), usize_len), borrow))
    }
}

impl Pointee for str {
    type Pointer = (u32, u32);

    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        <[u8]>::debug(pointer, f)
    }

    unsafe fn validate(
        ptr: Self::Pointer,
        borrow_mut: bool,
        mem: &dyn GuestMemory,
    ) -> Result<(*mut Self, BorrowHandle), GuestError> {
        let (raw, borrow) = <[u8]>::validate(ptr, borrow_mut, mem)?;

        // After we get a window into the raw view of bytes we use
        // `str::from_utf8` to validate the bytes to ensure they're utf-8. Note
        // that in the failure case we need to release the borrow.
        match str::from_utf8(&*raw) {
            Ok(s) => Ok((s as *const _ as *mut str, borrow)),
            Err(e) => {
                if borrow_mut {
                    mem.mut_unborrow(borrow);
                } else {
                    mem.shared_unborrow(borrow);
                }
                Err(e.into())
            }
        }
    }
}
