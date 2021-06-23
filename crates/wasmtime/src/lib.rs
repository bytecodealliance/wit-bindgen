pub use witx_bindgen_wasmtime_impl::{export, import};

#[cfg(feature = "async")]
pub use async_trait::async_trait;
#[cfg(feature = "tracing-lib")]
pub use tracing_lib as tracing;
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

pub struct RawMemory {
    pub slice: *mut [u8],
}

// This type is threadsafe despite its internal pointer because it allows no
// safe access to the internal pointer. Consumers must uphold Send/Sync
// guarantees themselves.
unsafe impl Send for RawMemory {}
unsafe impl Sync for RawMemory {}

#[doc(hidden)]
pub mod rt {
    use crate::{Endian, Le};
    use std::mem;
    use wasmtime::*;

    pub trait RawMem {
        fn store<T: Endian>(&mut self, offset: i32, val: T) -> Result<(), Trap>;
        fn store_many<T: Endian>(&mut self, offset: i32, vals: &[T]) -> Result<(), Trap>;
        fn load<T: Endian>(&self, offset: i32) -> Result<T, Trap>;
    }

    impl RawMem for [u8] {
        fn store<T: Endian>(&mut self, offset: i32, val: T) -> Result<(), Trap> {
            let mem = self
                .get_mut(offset as usize..)
                .and_then(|m| m.get_mut(..mem::size_of::<T>()))
                .ok_or_else(|| Trap::new("out of bounds write"))?;
            Le::from_slice_mut(mem)[0].set(val);
            Ok(())
        }

        fn store_many<T: Endian>(&mut self, offset: i32, val: &[T]) -> Result<(), Trap> {
            let mem = self
                .get_mut(offset as usize..)
                .and_then(|m| {
                    let len = mem::size_of::<T>().checked_mul(val.len())?;
                    m.get_mut(..len)
                })
                .ok_or_else(|| Trap::new("out of bounds write"))?;
            for (slot, val) in Le::from_slice_mut(mem).iter_mut().zip(val) {
                slot.set(*val);
            }
            Ok(())
        }

        fn load<T: Endian>(&self, offset: i32) -> Result<T, Trap> {
            let mem = self
                .get(offset as usize..)
                .and_then(|m| m.get(..mem::size_of::<Le<T>>()))
                .ok_or_else(|| Trap::new("out of bounds read"))?;
            Ok(Le::from_slice(mem)[0].get())
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

    pub fn copy_slice<T: Endian>(
        store: impl AsContextMut,
        memory: &Memory,
        free: &TypedFunc<(i32, i32, i32), ()>,
        base: i32,
        len: i32,
        align: i32,
    ) -> Result<Vec<T>, Trap> {
        let size = (len as u32)
            .checked_mul(mem::size_of::<T>() as u32)
            .ok_or_else(|| Trap::new("array too large to fit in wasm memory"))?;
        let slice = memory
            .data(&store)
            .get(base as usize..)
            .and_then(|s| s.get(..size as usize))
            .ok_or_else(|| Trap::new("out of bounds read"))?;
        let result = Le::from_slice(slice).iter().map(|s| s.get()).collect();
        free.call(store, (base, size as i32, align))?;
        Ok(result)
    }

    macro_rules! as_traits {
        ($(($name:ident $tr:ident $ty:ident ($($tys:ident)*)))*) => ($(
            pub fn $name<T: $tr>(t: T) -> $ty {
                t.$name()
            }

            pub trait $tr {
                fn $name(self) -> $ty;
            }

            impl<'a, T: Copy + $tr> $tr for &'a T {
                fn $name(self) -> $ty {
                    (*self).$name()
                }
            }

            $(
                impl $tr for $tys {
                    #[inline]
                    fn $name(self) -> $ty {
                        self as $ty
                    }
                }
            )*
        )*)
    }

    as_traits! {
        (as_i32 AsI32 i32 (char i8 u8 i16 u16 i32 u32))
        (as_i64 AsI64 i64 (i64 u64))
        (as_f32 AsF32 f32 (f32))
        (as_f64 AsF64 f64 (f64))
    }
}
