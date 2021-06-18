pub use witx_bindgen_wasmtime_impl::{export, import};

#[doc(hidden)]
pub use {anyhow, bitflags, wasmtime};

mod error;
pub mod exports;
pub mod imports;
mod le;
mod region;
mod table;

pub use error::GuestError;
pub use le::{Endian, Le};
pub use region::{AllBytesValid, BorrowChecker, Region};
pub use table::*;

#[doc(hidden)]
pub mod rt {
    use std::mem;
    use std::slice;
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

    pub fn copy_slice<T: crate::AllBytesValid>(
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
        unsafe {
            slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, size as usize)
                .copy_from_slice(slice);
            result.set_len(size as usize);
        }
        free.call(store, (base, size, align))?;
        Ok(result)
    }

    pub fn slice_as_bytes<T: crate::AllBytesValid>(slice: &[T]) -> &[u8] {
        unsafe { slice::from_raw_parts(slice.as_ptr() as *const u8, mem::size_of_val(slice)) }
    }

    pub fn as_i32<T: AsI32>(t: T) -> i32 {
        t.as_i32()
    }

    pub fn as_i64<T: AsI64>(t: T) -> i64 {
        t.as_i64()
    }

    pub trait AsI32 {
        fn as_i32(self) -> i32;
    }

    pub trait AsI64 {
        fn as_i64(self) -> i64;
    }

    impl<'a, T: Copy + AsI32> AsI32 for &'a T {
        fn as_i32(self) -> i32 {
            (*self).as_i32()
        }
    }

    impl<'a, T: Copy + AsI64> AsI64 for &'a T {
        fn as_i64(self) -> i64 {
            (*self).as_i64()
        }
    }

    macro_rules! as_i32 {
        ($($i:ident)*) => ($(
            impl AsI32 for $i {
                #[inline]
                fn as_i32(self) -> i32 {
                    self as i32
                }
            }
        )*)
    }

    as_i32!(char i8 u8 i16 u16 i32 u32);

    macro_rules! as_i64 {
        ($($i:ident)*) => ($(
            impl AsI64 for $i {
                #[inline]
                fn as_i64(self) -> i64 {
                    self as i64
                }
            }
        )*)
    }

    as_i64!(i64 u64);
}
