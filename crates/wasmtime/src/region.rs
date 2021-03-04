use crate::{BorrowHandle, GuestError};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
pub struct BorrowChecker {
    /// Unfortunately, since the terminology of std::cell and the problem domain of borrow checking
    /// overlap, the method calls on this member will be confusing.
    bc: RefCell<InnerBorrowChecker>,
}

impl BorrowChecker {
    /// A `BorrowChecker` manages run-time validation of borrows from a
    /// `GuestMemory`. It keeps track of regions of guest memory which are
    /// possible to alias with Rust references (via the `GuestSlice` and
    /// `GuestStr` structs, which implement `std::ops::Deref` and
    /// `std::ops::DerefMut`. It also enforces that `GuestPtr::read`
    /// does not access memory with an outstanding mutable borrow, and
    /// `GuestPtr::write` does not access memory with an outstanding
    /// shared or mutable borrow.
    pub fn new() -> Self {
        BorrowChecker::default()
    }
    /// Indicates whether any outstanding shared or mutable borrows are known
    /// to the `BorrowChecker`. This function must be `false` in order for it
    /// to be safe to recursively call into a WebAssembly module, or to
    /// manipulate the WebAssembly memory by any other means.
    pub fn has_outstanding_borrows(&self) -> bool {
        self.bc.borrow().has_outstanding_borrows()
    }
    pub fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.borrow_mut().shared_borrow(r)
    }
    pub fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.borrow_mut().mut_borrow(r)
    }
    pub fn shared_unborrow(&self, h: BorrowHandle) {
        self.bc.borrow_mut().shared_unborrow(h)
    }
    pub fn mut_unborrow(&self, h: BorrowHandle) {
        self.bc.borrow_mut().mut_unborrow(h)
    }
    pub fn is_shared_borrowed(&self, r: Region) -> bool {
        self.bc.borrow().is_shared_borrowed(r)
    }
    pub fn is_mut_borrowed(&self, r: Region) -> bool {
        self.bc.borrow().is_mut_borrowed(r)
    }
}

#[derive(Default, Debug)]
/// This is a pretty naive way to account for borrows. This datastructure
/// could be made a lot more efficient with some effort.
struct InnerBorrowChecker {
    /// Maps from handle to region borrowed. A HashMap is probably not ideal
    /// for this but it works. It would be more efficient if we could
    /// check `is_borrowed` without an O(n) iteration, by organizing borrows
    /// by an ordering of Region.
    shared_borrows: HashMap<BorrowHandle, Region>,
    mut_borrows: HashMap<BorrowHandle, Region>,
    /// Handle to give out for the next borrow. This is the bare minimum of
    /// bookkeeping of free handles, and in a pathological case we could run
    /// out, hence [`GuestError::BorrowCheckerOutOfHandles`]
    next_handle: BorrowHandle,
}

impl InnerBorrowChecker {
    fn has_outstanding_borrows(&self) -> bool {
        !(self.shared_borrows.is_empty() && self.mut_borrows.is_empty())
    }

    fn is_shared_borrowed(&self, r: Region) -> bool {
        self.shared_borrows.values().any(|b| b.overlaps(r))
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        self.mut_borrows.values().any(|b| b.overlaps(r))
    }

    fn new_handle(&mut self) -> Result<BorrowHandle, GuestError> {
        // Reset handles to 0 if all handles have been returned.
        if self.shared_borrows.is_empty() && self.mut_borrows.is_empty() {
            self.next_handle = BorrowHandle(0);
        }
        let h = self.next_handle;
        // Get the next handle. Since we don't recycle handles until all of
        // them have been returned, there is a pathological case where a user
        // may make a Very Large (usize::MAX) number of valid borrows and
        // unborrows while always keeping at least one borrow outstanding, and
        // we will run out of borrow handles.
        self.next_handle = BorrowHandle(
            h.0.checked_add(1)
                .ok_or_else(|| GuestError::BorrowCheckerOutOfHandles)?,
        );
        Ok(h)
    }

    fn shared_borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {
        if self.is_mut_borrowed(r) {
            return Err(GuestError::PtrBorrowed(r));
        }
        let h = self.new_handle()?;
        self.shared_borrows.insert(h, r);
        Ok(h)
    }

    fn mut_borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {
        if self.is_shared_borrowed(r) || self.is_mut_borrowed(r) {
            return Err(GuestError::PtrBorrowed(r));
        }
        let h = self.new_handle()?;
        self.mut_borrows.insert(h, r);
        Ok(h)
    }

    fn shared_unborrow(&mut self, h: BorrowHandle) {
        let removed = self.shared_borrows.remove(&h);
        debug_assert!(removed.is_some(), "double-freed shared borrow");
    }

    fn mut_unborrow(&mut self, h: BorrowHandle) {
        let removed = self.mut_borrows.remove(&h);
        debug_assert!(removed.is_some(), "double-freed mut borrow");
    }
}

/// Represents a contiguous region in memory.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    pub fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }

    /// Checks if this `Region` overlaps with `rhs` `Region`.
    pub fn overlaps(&self, rhs: Region) -> bool {
        // Zero-length regions can never overlap!
        if self.len == 0 || rhs.len == 0 {
            return false;
        }

        let self_start = self.start as u64;
        let self_end = self_start + (self.len - 1) as u64;

        let rhs_start = rhs.start as u64;
        let rhs_end = rhs_start + (rhs.len - 1) as u64;

        if self_start <= rhs_start {
            self_end >= rhs_start
        } else {
            rhs_end >= self_start
        }
    }

    pub fn extend(&self, times: u32) -> Self {
        let len = self.len * times;
        Self {
            start: self.start,
            len,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nonoverlapping() {
        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        bs.mut_borrow(r1).expect("can borrow r1");
        bs.mut_borrow(r2).expect("can borrow r2");

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
        bs.mut_borrow(r1).expect("can borrow r1");
        bs.mut_borrow(r2).expect("can borrow r2");
    }

    #[test]
    fn overlapping() {
        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));
        bs.shared_borrow(r1).expect("can borrow r1");
        assert!(bs.mut_borrow(r2).is_err(), "cant mut borrow r2");
        bs.shared_borrow(r2).expect("can shared borrow r2");

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));
        bs.shared_borrow(r1).expect("can borrow r1");
        assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
        bs.shared_borrow(r2).expect("can shared borrow r2");

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.shared_borrow(r1).expect("can borrow r1");
        assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
        bs.shared_borrow(r2).expect("can shared borrow r2");

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
        bs.shared_borrow(r1).expect("can borrow r1");
        assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
        bs.shared_borrow(r2).expect("can shared borrow r2");

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(2, 5);
        let r2 = Region::new(10, 5);
        let r3 = Region::new(15, 5);
        let r4 = Region::new(0, 10);
        assert!(r1.overlaps(r4));
        bs.shared_borrow(r1).expect("can borrow r1");
        bs.shared_borrow(r2).expect("can borrow r2");
        bs.shared_borrow(r3).expect("can borrow r3");
        assert!(bs.mut_borrow(r4).is_err(), "cant mut borrow r4");
        bs.shared_borrow(r4).expect("can shared borrow r4");
    }

    #[test]
    fn unborrowing() {
        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        assert_eq!(bs.has_outstanding_borrows(), false, "start with no borrows");
        let h1 = bs.mut_borrow(r1).expect("can borrow r1");
        assert_eq!(bs.has_outstanding_borrows(), true, "h1 is outstanding");
        let h2 = bs.mut_borrow(r2).expect("can borrow r2");

        assert!(bs.mut_borrow(r2).is_err(), "can't borrow r2 twice");
        bs.mut_unborrow(h2);
        assert_eq!(
            bs.has_outstanding_borrows(),
            true,
            "h1 is still outstanding"
        );
        bs.mut_unborrow(h1);
        assert_eq!(bs.has_outstanding_borrows(), false, "no remaining borrows");

        let _h3 = bs
            .mut_borrow(r2)
            .expect("can borrow r2 again now that its been unborrowed");

        // Lets try again with shared:

        let mut bs = InnerBorrowChecker::default();
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));
        assert_eq!(bs.has_outstanding_borrows(), false, "start with no borrows");
        let h1 = bs.shared_borrow(r1).expect("can borrow r1");
        assert_eq!(bs.has_outstanding_borrows(), true, "h1 is outstanding");
        let h2 = bs.shared_borrow(r2).expect("can borrow r2");
        let h3 = bs.shared_borrow(r2).expect("can shared borrow r2 twice");

        bs.shared_unborrow(h2);
        assert_eq!(
            bs.has_outstanding_borrows(),
            true,
            "h1, h3 still outstanding"
        );
        bs.shared_unborrow(h1);
        bs.shared_unborrow(h3);
        assert_eq!(bs.has_outstanding_borrows(), false, "no remaining borrows");
    }

    #[test]
    fn zero_length() {
        let r1 = Region::new(0, 0);
        let r2 = Region::new(0, 1);
        assert!(!r1.overlaps(r2));

        let r1 = Region::new(0, 1);
        let r2 = Region::new(0, 0);
        assert!(!r1.overlaps(r2));
    }

    #[test]
    fn nonoverlapping_region() {
        let r1 = Region::new(0, 10);
        let r2 = Region::new(10, 10);
        assert!(!r1.overlaps(r2));

        let r1 = Region::new(10, 10);
        let r2 = Region::new(0, 10);
        assert!(!r1.overlaps(r2));
    }

    #[test]
    fn overlapping_region() {
        let r1 = Region::new(0, 10);
        let r2 = Region::new(9, 10);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(0, 10);
        let r2 = Region::new(2, 5);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(9, 10);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));

        let r1 = Region::new(2, 5);
        let r2 = Region::new(0, 10);
        assert!(r1.overlaps(r2));
    }
}
