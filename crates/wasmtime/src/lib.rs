pub use witx_bindgen_wasmtime_impl::{export, import};

#[doc(hidden)]
pub use {anyhow, bitflags, wasmtime};

mod error;
pub mod exports;
pub mod imports;
mod memory;
mod ptr;
mod region;
mod table;

pub use error::GuestError;
pub use memory::{BorrowHandle, GuestMemory};
pub use ptr::{GuestPtr, Pointee};
pub use region::{BorrowChecker, Region};
pub use table::*;

// pub struct WasmtimeGuestMemory<'a> {
//     mem: &'a wasmtime::Memory,
//     borrows: &'a BorrowChecker,
// }

// impl<'a> WasmtimeGuestMemory<'a> {
//     // Unsafe because we don't know that `borrows` are valid for `mem
//     pub unsafe fn new(
//         mem: &'a wasmtime::Memory,
//         borrows: &'a BorrowChecker,
//     ) -> WasmtimeGuestMemory<'a> {
//         WasmtimeGuestMemory { mem, borrows }
//     }
// }

// unsafe impl<'a> GuestMemory for WasmtimeGuestMemory<'a> {
//     fn base(&self) -> (*mut u8, u32) {
//         (self.mem.data_ptr(), self.mem.data_size() as u32)
//     }
//     fn has_outstanding_borrows(&self) -> bool {
//         self.borrows.has_outstanding_borrows()
//     }
//     fn is_mut_borrowed(&self, r: Region) -> bool {
//         self.borrows.is_mut_borrowed(r)
//     }
//     fn is_shared_borrowed(&self, r: Region) -> bool {
//         self.borrows.is_shared_borrowed(r)
//     }
//     fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
//         self.borrows.mut_borrow(r)
//     }
//     fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
//         self.borrows.shared_borrow(r)
//     }
//     fn mut_unborrow(&self, h: BorrowHandle) {
//         self.borrows.mut_unborrow(h)
//     }
//     fn shared_unborrow(&self, h: BorrowHandle) {
//         self.borrows.shared_unborrow(h)
//     }
// }

#[doc(hidden)]
pub mod rt {
    use std::mem;
    use wasmtime::*;

    pub trait RawMem {
        fn store(&mut self, offset: i32, bytes: &[u8]) -> Result<(), Trap>;
        fn load<T: AsMut<[u8]>, U>(
            &self,
            offset: i32,
            bytes: T,
            cvt: impl FnOnce(T) -> U,
        ) -> Result<U, Trap>;
    }

    impl RawMem for [u8] {
        fn store(&mut self, offset: i32, bytes: &[u8]) -> Result<(), Trap> {
            let mem = self
                .get_mut(offset as usize..)
                .and_then(|m| m.get_mut(..bytes.len()))
                .ok_or_else(|| Trap::new("out of bounds write"))?;
            mem.copy_from_slice(bytes);
            Ok(())
        }

        fn load<T: AsMut<[u8]>, U>(
            &self,
            offset: i32,
            mut bytes: T,
            cvt: impl FnOnce(T) -> U,
        ) -> Result<U, Trap> {
            let dst = bytes.as_mut();
            let mem = self
                .get(offset as usize..)
                .and_then(|m| m.get(..dst.len()))
                .ok_or_else(|| Trap::new("out of bounds read"))?;
            dst.copy_from_slice(mem);
            Ok(cvt(bytes))
        }
    }

    pub fn char_from_i32(val: i32) -> Result<char, Trap> {
        core::char::from_u32(val as u32).ok_or_else(|| Trap::new("char value out of valid range"))
    }

    pub fn invalid_variant(name: &str) -> Trap {
        let msg = format!("invalid discriminant for `{}`", name);
        Trap::new(msg)
    }

    pub fn validate_flags<U>(
        bits: i64,
        all: i64,
        name: &str,
        mk: impl FnOnce(i64) -> U,
    ) -> Result<U, Trap> {
        if bits & !all != 0 {
            let msg = format!("invalid flags specified for `{}`", name);
            Err(Trap::new(msg))
        } else {
            Ok(mk(bits))
        }
    }

    pub fn data_and_memory<'a, T>(
        mut caller: &'a mut Caller<'_, T>,
        memory: &Memory,
    ) -> (&'a mut [u8], &'a mut T) {
        // TODO: comment unsafe
        unsafe {
            let memory = &mut *(memory.data_mut(&mut caller) as *mut [u8]);
            (memory, caller.data_mut())
        }
    }

    pub fn get_func<T>(caller: &mut Caller<'_, T>, func: &str) -> Result<Func, wasmtime::Trap> {
        let func = caller
            .get_export(func)
            .ok_or_else(|| {
                let msg = format!("`{}` export not available", func);
                Trap::new(msg)
            })?
            .into_func()
            .ok_or_else(|| {
                let msg = format!("`{}` export not a function", func);
                Trap::new(msg)
            })?;
        Ok(func)
    }

    pub fn get_memory<T>(caller: &mut Caller<'_, T>, mem: &str) -> Result<Memory, wasmtime::Trap> {
        let mem = caller
            .get_export(mem)
            .ok_or_else(|| {
                let msg = format!("`{}` export not available", mem);
                Trap::new(msg)
            })?
            .into_memory()
            .ok_or_else(|| {
                let msg = format!("`{}` export not a memory", mem);
                Trap::new(msg)
            })?;
        Ok(mem)
    }

    pub fn bad_int(_: std::num::TryFromIntError) -> Trap {
        let msg = "out-of-bounds integer conversion";
        Trap::new(msg)
    }

    pub unsafe fn copy_slice<T: Copy>(
        store: impl AsContextMut,
        memory: &Memory,
        free: &TypedFunc<(i32, i32, i32), ()>,
        base: i32,
        len: i32,
        align: i32,
    ) -> Result<Vec<T>, Trap> {
        let mut result = Vec::with_capacity(len as usize);
        let size = len * (mem::size_of::<T>() as i32);
        let slice = memory
            .data(&store)
            .get(base as usize..)
            .and_then(|s| s.get(..size as usize))
            .ok_or_else(|| Trap::new("out of bounds read"))?;
        std::slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, size as usize)
            .copy_from_slice(slice);
        result.set_len(size as usize);
        free.call(store, (base, size, align))?;
        Ok(result)
    }
}
