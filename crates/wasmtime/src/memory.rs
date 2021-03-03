use crate::{GuestError, Region};
use std::rc::Rc;
use std::sync::Arc;

/// A trait which abstracts how to get at the region of host memory taht
/// contains guest memory.
///
/// All `GuestPtr` types will contain a handle to this trait, signifying where
/// the pointer is actually pointing into. This type will need to be implemented
/// for the host's memory storage object.
///
/// # Safety
///
/// Safety around this type is tricky, and the trait is `unsafe` since there are
/// a few contracts you need to uphold to implement this type correctly and have
/// everything else in this crate work out safely.
///
/// The most important method of this trait is the `base` method. This returns,
/// in host memory, a pointer and a length. The pointer should point to valid
/// memory for the guest to read/write for the length contiguous bytes
/// afterwards.
///
/// The region returned by `base` must not only be valid, however, but it must
/// be valid for "a period of time before the guest is reentered". This isn't
/// exactly well defined but the general idea is that `GuestMemory` is allowed
/// to change under our feet to accomodate instructions like `memory.grow` or
/// other guest modifications. Memory, however, cannot be changed if the guest
/// is not reentered or if no explicitly action is taken to modify the guest
/// memory.
///
/// This provides the guarantee that host pointers based on the return value of
/// `base` have a dynamic period for which they are valid. This time duration
/// must be "somehow nonzero in length" to allow users of `GuestMemory` and
/// `GuestPtr` to safely read and write interior data.
///
/// This type also provides methods for run-time borrow checking of references
/// into the memory. The safety of this mechanism depends on there being
/// exactly one associated tracking of borrows for a given WebAssembly memory.
/// There must be no other reads or writes of WebAssembly the memory by either
/// Rust or WebAssembly code while there are any outstanding borrows, as given
/// by `GuestMemory::has_outstanding_borrows()`.
///
/// # Using References
///
/// The [`GuestPtr::as_slice`] or [`GuestPtr::as_str`] will return smart
/// pointers [`GuestSlice`] and [`GuestStr`]. These types, which implement
/// [`std::ops::Deref`] and [`std::ops::DerefMut`], provide mutable references
/// into the memory region given by a `GuestMemory`.
///
/// These smart pointers are dynamically borrow-checked by the borrow checker
/// methods on this trait. While a `GuestSlice` or a `GuestStr` are live, the
/// [`GuestMemory::has_outstanding_borrows()`] method will always return
/// `true`. If you need to re-enter the guest or otherwise read or write to
/// the contents of a WebAssembly memory, all `GuestSlice`s and `GuestStr`s
/// for the memory must be dropped, at which point
/// `GuestMemory::has_outstanding_borrows()` will return `false`.
pub unsafe trait GuestMemory {
    /// Returns the base allocation of this guest memory, located in host
    /// memory.
    ///
    /// A pointer/length pair are returned to signify where the guest memory
    /// lives in the host, and how many contiguous bytes the memory is valid for
    /// after the returned pointer.
    ///
    /// Note that there are safety guarantees about this method that
    /// implementations must uphold, and for more details see the
    /// [`GuestMemory`] documentation.
    fn base(&self) -> (*mut u8, u32);

    /// Validates a guest-relative pointer given various attributes, and returns
    /// the corresponding host pointer.
    ///
    /// * `offset` - this is the guest-relative pointer, an offset from the
    ///   base.
    /// * `align` - this is the desired alignment of the guest pointer, and if
    ///   successful the host pointer will be guaranteed to have this alignment.
    /// * `len` - this is the number of bytes, after `offset`, that the returned
    ///   pointer must be valid for.
    ///
    /// This function will guarantee that the returned pointer is in-bounds of
    /// `base`, *at this time*, for `len` bytes and has alignment `align`. If
    /// any guarantees are not upheld then an error will be returned.
    ///
    /// Note that the returned pointer is an unsafe pointer. This is not safe to
    /// use in general because guest memory can be relocated. Additionally the
    /// guest may be modifying/reading memory as well. Consult the
    /// [`GuestMemory`] documentation for safety information about using this
    /// returned pointer.
    fn validate_size_align(&self, region: Region, align: usize) -> Result<*mut u8, GuestError> {
        let (base_ptr, base_len) = self.base();

        // Figure out our pointer to the start of memory
        let start = match (base_ptr as usize).checked_add(region.start as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and use that to figure out the end pointer
        let end = match start.checked_add(region.len as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and then verify that our end doesn't reach past the end of our memory
        if end > (base_ptr as usize) + (base_len as usize) {
            return Err(GuestError::PtrOutOfBounds(region));
        }
        // and finally verify that the alignment is correct
        if start % align != 0 {
            return Err(GuestError::PtrNotAligned(region, align as u32));
        }
        Ok(start as *mut u8)
    }

    /// Indicates whether any outstanding borrows are known to the
    /// `GuestMemory`. This function must be `false` in order for it to be
    /// safe to recursively call into a WebAssembly module, or to manipulate
    /// the WebAssembly memory by any other means.
    fn has_outstanding_borrows(&self) -> bool;
    /// Check if a region of linear memory is exclusively borrowed. This is called during any
    /// `GuestPtr::read` or `GuestPtr::write` operation to ensure that wiggle is not reading or
    /// writing a region of memory which Rust believes it has exclusive access to.
    fn is_mut_borrowed(&self, r: Region) -> bool;
    /// Check if a region of linear memory has any shared borrows.
    fn is_shared_borrowed(&self, r: Region) -> bool;
    /// Exclusively borrow a region of linear memory. This is used when constructing a
    /// `GuestSliceMut` or `GuestStrMut`. Those types will give Rust `&mut` access
    /// to the region of linear memory, therefore, the `GuestMemory` impl must
    /// guarantee that at most one `BorrowHandle` is issued to a given region,
    /// `GuestMemory::has_outstanding_borrows` is true for the duration of the
    /// borrow, and that `GuestMemory::is_mut_borrowed` of any overlapping region
    /// is false for the duration of the borrow.
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;
    /// Shared borrow a region of linear memory. This is used when constructing a
    /// `GuestSlice` or `GuestStr`. Those types will give Rust `&` (shared reference) access
    /// to the region of linear memory.
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;
    /// Unborrow a previously borrowed mutable region. As long as `GuestSliceMut` and
    /// `GuestStrMut` are implemented correctly, a mut `BorrowHandle` should only be
    /// unborrowed once.
    fn mut_unborrow(&self, h: BorrowHandle);
    /// Unborrow a previously borrowed shared region. As long as `GuestSlice`
    /// and `GuestStr` are implemented correctly, a shared `BorrowHandle` should
    /// only be unborrowed once.
    fn shared_unborrow(&self, h: BorrowHandle);
}

/// A handle to a borrow on linear memory. It is produced by `{mut,
/// shared}_borrow` and consumed by `{mut, shared}_unborrow`. Only the
/// `GuestMemory` impl should ever construct a `BorrowHandle` or inspect its
/// contents.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BorrowHandle(pub usize);

// Forwarding trait implementations to the original type
unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a mut T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Box<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Rc<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Arc<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}
