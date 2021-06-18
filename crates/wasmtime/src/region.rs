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

    pub fn slice<T: AllBytesValid>(&mut self, ptr: i32, len: i32) -> Result<&'a [T], Trap> {
        let (ret, r) = self.get_slice(ptr, len)?;
        self.shared_borrows.insert(r);
        Ok(ret)
    }

    pub fn slice_mut<T: AllBytesValid>(&mut self, ptr: i32, len: i32) -> Result<&'a mut [T], Trap> {
        let (ret, r) = self.get_slice_mut(ptr, len)?;
        self.mut_borrows.insert(r);
        Ok(ret)
    }

    fn get_slice<T: AllBytesValid>(&self, ptr: i32, len: i32) -> Result<(&'a [T], Region), Trap> {
        let r = self.region::<T>(ptr, len)?;
        if self.is_mut_borrowed(r) {
            Err(to_trap(GuestError::PtrBorrowed(r)))
        } else {
            Ok((
                unsafe {
                    std::slice::from_raw_parts(
                        self.ptr.add(r.start as usize) as *const T,
                        len as usize,
                    )
                },
                r,
            ))
        }
    }

    fn get_slice_mut<T>(&mut self, ptr: i32, len: i32) -> Result<(&'a mut [T], Region), Trap> {
        let r = self.region::<T>(ptr, len)?;
        if self.is_mut_borrowed(r) || self.is_shared_borrowed(r) {
            Err(to_trap(GuestError::PtrBorrowed(r)))
        } else {
            Ok((
                unsafe {
                    std::slice::from_raw_parts_mut(
                        self.ptr.add(r.start as usize) as *mut T,
                        len as usize,
                    )
                },
                r,
            ))
        }
    }

    fn region<T>(&self, ptr: i32, len: i32) -> Result<Region, Trap> {
        assert!(std::mem::align_of::<T>() == 1);
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
        let bytes = self.slice(ptr, len)?;
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

    fn is_shared_borrowed(&self, r: Region) -> bool {
        self.shared_borrows.iter().any(|b| b.overlaps(r))
    }

    fn is_mut_borrowed(&self, r: Region) -> bool {
        self.mut_borrows.iter().any(|b| b.overlaps(r))
    }

    pub fn raw(&self) -> *mut [u8] {
        std::ptr::slice_from_raw_parts_mut(self.ptr, self.len)
    }
}

impl crate::rt::RawMem for BorrowChecker<'_> {
    fn store(&mut self, offset: i32, bytes: &[u8]) -> Result<(), Trap> {
        let (slice, _) = self.get_slice_mut::<u8>(offset, bytes.len() as i32)?;
        slice.copy_from_slice(bytes);
        Ok(())
    }

    fn load<T: AsMut<[u8]>, U>(
        &self,
        offset: i32,
        mut bytes: T,
        cvt: impl FnOnce(T) -> U,
    ) -> Result<U, Trap> {
        let (slice, _) = self.get_slice::<u8>(offset, bytes.as_mut().len() as i32)?;
        bytes.as_mut().copy_from_slice(slice);
        Ok(cvt(bytes))
    }
}

/// Unsafe trait representing types where every byte pattern is valid for their
/// representation.
///
/// This is the set of types which wasmtime can have a raw pointer to for
/// values which reside in wasm linear memory.
pub unsafe trait AllBytesValid {}

unsafe impl AllBytesValid for u8 {}
unsafe impl AllBytesValid for u16 {}
unsafe impl AllBytesValid for u32 {}
unsafe impl AllBytesValid for u64 {}
unsafe impl AllBytesValid for i8 {}
unsafe impl AllBytesValid for i16 {}
unsafe impl AllBytesValid for i32 {}
unsafe impl AllBytesValid for i64 {}
unsafe impl AllBytesValid for f32 {}
unsafe impl AllBytesValid for f64 {}

macro_rules! tuples {
    ($(($($t:ident)*))*) => ($(
        unsafe impl <$($t:AllBytesValid,)*> AllBytesValid for ($($t,)*) {}
    )*)
}

tuples! {
    ()
    (T1)
    (T1 T2)
    (T1 T2 T3)
    (T1 T2 T3 T4)
    (T1 T2 T3 T4 T5)
    (T1 T2 T3 T4 T5 T6)
    (T1 T2 T3 T4 T5 T6 T7)
    (T1 T2 T3 T4 T5 T6 T7 T8)
    (T1 T2 T3 T4 T5 T6 T7 T8 T9)
    (T1 T2 T3 T4 T5 T6 T7 T8 T9 T10)
}

/// Represents a contiguous region in memory.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    /// Checks if this `Region` overlaps with `rhs` `Region`.
    fn overlaps(&self, rhs: Region) -> bool {
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nonoverlapping() {
        let mut bytes = [0; 100];
        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(0, 10).unwrap();
        bc.slice::<u8>(10, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(10, 10).unwrap();
        bc.slice::<u8>(0, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice_mut::<u8>(0, 10).unwrap();
        bc.slice_mut::<u8>(10, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice_mut::<u8>(10, 10).unwrap();
        bc.slice_mut::<u8>(0, 10).unwrap();
    }

    #[test]
    fn overlapping() {
        let mut bytes = [0; 100];
        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(0, 10).unwrap();
        bc.slice_mut::<u8>(9, 10).unwrap_err();
        bc.slice::<u8>(9, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(0, 10).unwrap();
        bc.slice_mut::<u8>(2, 5).unwrap_err();
        bc.slice::<u8>(2, 5).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(9, 10).unwrap();
        bc.slice_mut::<u8>(0, 10).unwrap_err();
        bc.slice::<u8>(0, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(2, 5).unwrap();
        bc.slice_mut::<u8>(0, 10).unwrap_err();
        bc.slice::<u8>(0, 10).unwrap();

        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice::<u8>(2, 5).unwrap();
        bc.slice::<u8>(10, 5).unwrap();
        bc.slice::<u8>(15, 5).unwrap();
        bc.slice_mut::<u8>(0, 10).unwrap_err();
        bc.slice::<u8>(0, 10).unwrap();
    }

    #[test]
    fn zero_length() {
        let mut bytes = [0; 100];
        let mut bc = BorrowChecker::new(&mut bytes);
        bc.slice_mut::<u8>(0, 0).unwrap();
        bc.slice_mut::<u8>(0, 0).unwrap();
        bc.slice::<u8>(0, 1).unwrap();
    }
}
