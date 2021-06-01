use crate::GuestError;
use std::collections::HashSet;
use std::marker;
use std::mem;
use wasmtime::Trap;

// This is a pretty naive way to account for borrows. This datastructure
// could be made a lot more efficient with some effort.
pub struct BorrowChecker<'a> {
    /// Maps from handle to region borrowed. A HashMap is probably not ideal
    /// for this but it works. It would be more efficient if we could
    /// check `is_borrowed` without an O(n) iteration, by organizing borrows
    /// by an ordering of Region.
    shared_borrows: HashSet<Region>,
    mut_borrows: HashSet<Region>,
    _marker: marker::PhantomData<&'a mut [u8]>,
    ptr: *mut u8,
    len: usize,
}

// unsafe impl Send for BorrowChecker<'_> {}
// unsafe impl Sync for BorrowChecker<'_> {}

fn to_trap(err: impl std::error::Error + Send + Sync + 'static) -> Trap {
    Trap::from(Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
}

impl<'a> BorrowChecker<'a> {
    pub fn new(data: &'a mut [u8]) -> BorrowChecker<'a> {
        BorrowChecker {
            ptr: data.as_mut_ptr(),
            len: data.len(),
            shared_borrows: Default::default(),
            mut_borrows: Default::default(),
            _marker: marker::PhantomData,
        }
    }

    pub unsafe fn slice<T>(&mut self, ptr: i32, len: i32) -> Result<&'a [T], Trap> {
        let (ret, r) = self.get_slice(ptr, len)?;
        self.shared_borrows.insert(r);
        Ok(ret)
    }

    pub unsafe fn slice_mut<T>(&mut self, ptr: i32, len: i32) -> Result<&'a mut [T], Trap> {
        let (ret, r) = self.get_slice_mut(ptr, len)?;
        self.mut_borrows.insert(r);
        Ok(ret)
    }

    unsafe fn get_slice<T>(&self, ptr: i32, len: i32) -> Result<(&'a [T], Region), Trap> {
        let r = self.region::<T>(ptr, len)?;
        if self.is_mut_borrowed(r) {
            Err(to_trap(GuestError::PtrBorrowed(r)))
        } else {
            Ok((
                std::slice::from_raw_parts(
                    self.ptr.add(r.start as usize) as *const T,
                    len as usize,
                ),
                r,
            ))
        }
    }

    unsafe fn get_slice_mut<T>(
        &mut self,
        ptr: i32,
        len: i32,
    ) -> Result<(&'a mut [T], Region), Trap> {
        let r = self.region::<T>(ptr, len)?;
        if self.is_mut_borrowed(r) || self.is_shared_borrowed(r) {
            Err(to_trap(GuestError::PtrBorrowed(r)))
        } else {
            Ok((
                std::slice::from_raw_parts_mut(
                    self.ptr.add(r.start as usize) as *mut T,
                    len as usize,
                ),
                r,
            ))
        }
    }

    fn region<T>(&self, ptr: i32, len: i32) -> Result<Region, Trap> {
        assert_eq!(std::mem::align_of::<T>(), 1);
        let r = Region {
            start: ptr as u32,
            len: (len as u32)
                .checked_mul(mem::size_of::<T>() as u32)
                .ok_or_else(|| to_trap(GuestError::PtrOverflow))?,
        };
        self.validate_contains(&r)?;
        Ok(r)
    }

    pub fn slice_str(&mut self, ptr: i32, len: i32) -> Result<&'a str, Trap> {
        let bytes = unsafe { self.slice::<u8>(ptr, len)? };
        std::str::from_utf8(bytes).map_err(to_trap)
    }

    fn validate_contains(&self, region: &Region) -> Result<(), Trap> {
        let end = region
            .start
            .checked_add(region.len)
            .ok_or_else(|| to_trap(GuestError::PtrOverflow))? as usize;
        if end <= self.len {
            Ok(())
        } else {
            Err(to_trap(GuestError::PtrOutOfBounds(*region)))
        }
    }

    // pub unsafe fn shared_borrow<T>(&mut self, ptr: i32) -> Result<(), GuestError> {
    //     if self.is_mut_borrowed(r) {
    //         Err(GuestError::PtrBorrowed(r))
    //     } else {
    //         self.shared_borrows.insert(r);
    //         Ok(())
    //     }
    // }

    // pub unsafe fn mut_borrow(&mut self, r: Region) -> Result<(), GuestError> {
    //     if self.is_shared_borrowed(r) || self.is_mut_borrowed(r) {
    //         Err(GuestError::PtrBorrowed(r))
    //     } else {
    //         self.mut_borrows.insert(r);
    //         Ok(())
    //     }
    // }

    fn is_shared_borrowed(&self, r: Region) -> bool {
        self.shared_borrows.iter().any(|b| b.overlaps(r))
    }

    fn is_mut_borrowed(&self, r: Region) -> bool {
        self.mut_borrows.iter().any(|b| b.overlaps(r))
    }
}

impl crate::rt::RawMem for BorrowChecker<'_> {
    fn store(&mut self, offset: i32, bytes: &[u8]) -> Result<(), Trap> {
        unsafe {
            let (slice, _) = self.get_slice_mut::<u8>(offset, bytes.len() as i32)?;
            slice.copy_from_slice(bytes);
            Ok(())
        }
    }

    fn load<T: AsMut<[u8]>, U>(
        &self,
        offset: i32,
        mut bytes: T,
        cvt: impl FnOnce(T) -> U,
    ) -> Result<U, Trap> {
        unsafe {
            let (slice, _) = self.get_slice::<u8>(offset, bytes.as_mut().len() as i32)?;
            bytes.as_mut().copy_from_slice(slice);
            Ok(cvt(bytes))
        }
    }
}

// #[derive(Default, Debug)]
// struct InnerBorrowChecker {}

// impl InnerBorrowChecker {
//     fn has_outstanding_borrows(&self) -> bool {
//         !(self.shared_borrows.is_empty() && self.mut_borrows.is_empty())
//     }

//     fn is_shared_borrowed(&self, r: Region) -> bool {}
//     fn is_mut_borrowed(&self, r: Region) -> bool {}

//     fn new_handle(&mut self) -> Result<BorrowHandle, GuestError> {
//         // Reset handles to 0 if all handles have been returned.
//         if self.shared_borrows.is_empty() && self.mut_borrows.is_empty() {
//             self.next_handle = BorrowHandle(0);
//         }
//         let h = self.next_handle;
//         // Get the next handle. Since we don't recycle handles until all of
//         // them have been returned, there is a pathological case where a user
//         // may make a Very Large (usize::MAX) number of valid borrows and
//         // unborrows while always keeping at least one borrow outstanding, and
//         // we will run out of borrow handles.
//         self.next_handle = BorrowHandle(
//             h.0.checked_add(1)
//                 .ok_or_else(|| GuestError::BorrowCheckerOutOfHandles)?,
//         );
//         Ok(h)
//     }

//     fn shared_borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {}

//     fn mut_borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {
//         if self.is_shared_borrowed(r) || self.is_mut_borrowed(r) {
//             return Err(GuestError::PtrBorrowed(r));
//         }
//         let h = self.new_handle()?;
//         self.mut_borrows.insert(h, r);
//         Ok(h)
//     }

//     fn shared_unborrow(&mut self, h: BorrowHandle) {
//         let removed = self.shared_borrows.remove(&h);
//         debug_assert!(removed.is_some(), "double-freed shared borrow");
//     }

//     fn mut_unborrow(&mut self, h: BorrowHandle) {
//         let removed = self.mut_borrows.remove(&h);
//         debug_assert!(removed.is_some(), "double-freed mut borrow");
//     }
// }

/// Represents a contiguous region in memory.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    //     pub fn new(start: u32, len: u32) -> Self {
    //         Self { start, len }
    //     }

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

    //     pub fn extend(&self, times: u32) -> Self {
    //         let len = self.len * times;
    //         Self {
    //             start: self.start,
    //             len,
    //         }
    //     }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn nonoverlapping() {
//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(10, 10);
//         assert!(!r1.overlaps(r2));
//         bs.mut_borrow(r1).expect("can borrow r1");
//         bs.mut_borrow(r2).expect("can borrow r2");

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(10, 10);
//         let r2 = Region::new(0, 10);
//         assert!(!r1.overlaps(r2));
//         bs.mut_borrow(r1).expect("can borrow r1");
//         bs.mut_borrow(r2).expect("can borrow r2");
//     }

//     #[test]
//     fn overlapping() {
//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(9, 10);
//         assert!(r1.overlaps(r2));
//         bs.shared_borrow(r1).expect("can borrow r1");
//         assert!(bs.mut_borrow(r2).is_err(), "cant mut borrow r2");
//         bs.shared_borrow(r2).expect("can shared borrow r2");

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(2, 5);
//         assert!(r1.overlaps(r2));
//         bs.shared_borrow(r1).expect("can borrow r1");
//         assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
//         bs.shared_borrow(r2).expect("can shared borrow r2");

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(9, 10);
//         let r2 = Region::new(0, 10);
//         assert!(r1.overlaps(r2));
//         bs.shared_borrow(r1).expect("can borrow r1");
//         assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
//         bs.shared_borrow(r2).expect("can shared borrow r2");

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(2, 5);
//         let r2 = Region::new(0, 10);
//         assert!(r1.overlaps(r2));
//         bs.shared_borrow(r1).expect("can borrow r1");
//         assert!(bs.mut_borrow(r2).is_err(), "cant borrow r2");
//         bs.shared_borrow(r2).expect("can shared borrow r2");

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(2, 5);
//         let r2 = Region::new(10, 5);
//         let r3 = Region::new(15, 5);
//         let r4 = Region::new(0, 10);
//         assert!(r1.overlaps(r4));
//         bs.shared_borrow(r1).expect("can borrow r1");
//         bs.shared_borrow(r2).expect("can borrow r2");
//         bs.shared_borrow(r3).expect("can borrow r3");
//         assert!(bs.mut_borrow(r4).is_err(), "cant mut borrow r4");
//         bs.shared_borrow(r4).expect("can shared borrow r4");
//     }

//     #[test]
//     fn unborrowing() {
//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(10, 10);
//         assert!(!r1.overlaps(r2));
//         assert_eq!(bs.has_outstanding_borrows(), false, "start with no borrows");
//         let h1 = bs.mut_borrow(r1).expect("can borrow r1");
//         assert_eq!(bs.has_outstanding_borrows(), true, "h1 is outstanding");
//         let h2 = bs.mut_borrow(r2).expect("can borrow r2");

//         assert!(bs.mut_borrow(r2).is_err(), "can't borrow r2 twice");
//         bs.mut_unborrow(h2);
//         assert_eq!(
//             bs.has_outstanding_borrows(),
//             true,
//             "h1 is still outstanding"
//         );
//         bs.mut_unborrow(h1);
//         assert_eq!(bs.has_outstanding_borrows(), false, "no remaining borrows");

//         let _h3 = bs
//             .mut_borrow(r2)
//             .expect("can borrow r2 again now that its been unborrowed");

//         // Lets try again with shared:

//         let mut bs = InnerBorrowChecker::default();
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(10, 10);
//         assert!(!r1.overlaps(r2));
//         assert_eq!(bs.has_outstanding_borrows(), false, "start with no borrows");
//         let h1 = bs.shared_borrow(r1).expect("can borrow r1");
//         assert_eq!(bs.has_outstanding_borrows(), true, "h1 is outstanding");
//         let h2 = bs.shared_borrow(r2).expect("can borrow r2");
//         let h3 = bs.shared_borrow(r2).expect("can shared borrow r2 twice");

//         bs.shared_unborrow(h2);
//         assert_eq!(
//             bs.has_outstanding_borrows(),
//             true,
//             "h1, h3 still outstanding"
//         );
//         bs.shared_unborrow(h1);
//         bs.shared_unborrow(h3);
//         assert_eq!(bs.has_outstanding_borrows(), false, "no remaining borrows");
//     }

//     #[test]
//     fn zero_length() {
//         let r1 = Region::new(0, 0);
//         let r2 = Region::new(0, 1);
//         assert!(!r1.overlaps(r2));

//         let r1 = Region::new(0, 1);
//         let r2 = Region::new(0, 0);
//         assert!(!r1.overlaps(r2));
//     }

//     #[test]
//     fn nonoverlapping_region() {
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(10, 10);
//         assert!(!r1.overlaps(r2));

//         let r1 = Region::new(10, 10);
//         let r2 = Region::new(0, 10);
//         assert!(!r1.overlaps(r2));
//     }

//     #[test]
//     fn overlapping_region() {
//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(9, 10);
//         assert!(r1.overlaps(r2));

//         let r1 = Region::new(0, 10);
//         let r2 = Region::new(2, 5);
//         assert!(r1.overlaps(r2));

//         let r1 = Region::new(9, 10);
//         let r2 = Region::new(0, 10);
//         assert!(r1.overlaps(r2));

//         let r1 = Region::new(2, 5);
//         let r2 = Region::new(0, 10);
//         assert!(r1.overlaps(r2));
//     }
// }
