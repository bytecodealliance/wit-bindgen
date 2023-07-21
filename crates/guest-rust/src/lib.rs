#![no_std]

extern crate alloc;

#[cfg(feature = "macros")]
pub use wit_bindgen_rust_macro::*;

// Re-export `bitflags` so that we can reference it from macros.
#[doc(hidden)]
pub use bitflags;

#[doc(hidden)]
pub mod rt {
    use crate::alloc::string::String;
    use crate::alloc::vec::Vec;

    /// Provide a hook for generated export functions to run static
    /// constructors at most once. wit-bindgen-rust generates a call to this
    /// function at the start of all component export functions. Importantly,
    /// it is not called as part of `cabi_realloc`, which is a *core* export
    /// func, but may not execute ctors, because the environment ctor in
    /// wasi-libc (before rust 1.69.0) calls an import func, which is not
    /// permitted by the Component Model when inside realloc.
    ///
    /// We intend to remove this once rust 1.69.0 stabilizes.
    #[cfg(target_arch = "wasm32")]
    pub fn run_ctors_once() {
        static mut RUN: bool = false;
        unsafe {
            if !RUN {
                // This function is synthesized by `wasm-ld` to run all static
                // constructors. wasm-ld will either provide an implementation
                // of this symbol, or synthesize a wrapper around each
                // exported function to (unconditionally) run ctors. By using
                // this function, the linked module is opting into "manually"
                // running ctors.
                extern "C" {
                    fn __wasm_call_ctors();
                }
                __wasm_call_ctors();
                RUN = true;
            }
        }
    }

    use super::alloc::alloc::Layout;

    // Re-export things from liballoc for convenient use.
    pub use super::alloc::{alloc, boxed, string, vec};

    #[cfg(feature = "realloc")]
    #[no_mangle]
    unsafe extern "C" fn cabi_realloc(
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
            debug_assert_ne!(new_len, 0, "non-zero old_len requires non-zero new_len!");
            layout = Layout::from_size_align_unchecked(old_len, align);
            alloc::realloc(old_ptr, layout, new_len)
        };
        if ptr.is_null() {
            // Print a nice message in debug mode, but in release mode don't
            // pull in so many dependencies related to printing so just emit an
            // `unreachable` instruction.
            if cfg!(debug_assertions) {
                alloc::handle_alloc_error(layout);
            } else {
                #[cfg(target_arch = "wasm32")]
                core::arch::wasm32::unreachable();
                #[cfg(not(target_arch = "wasm32"))]
                unreachable!();
            }
        }
        return ptr;
    }

    pub unsafe fn dealloc(ptr: i32, size: usize, align: usize) {
        if size == 0 {
            return;
        }
        let layout = Layout::from_size_align_unchecked(size, align);
        alloc::dealloc(ptr as *mut u8, layout);
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

    pub unsafe fn string_lift(bytes: Vec<u8>) -> String {
        if cfg!(debug_assertions) {
            String::from_utf8(bytes).unwrap()
        } else {
            String::from_utf8_unchecked(bytes)
        }
    }

    pub unsafe fn invalid_enum_discriminant<T>() -> T {
        if cfg!(debug_assertions) {
            panic!("invalid enum discriminant")
        } else {
            core::hint::unreachable_unchecked()
        }
    }

    pub unsafe fn char_lift(val: u32) -> char {
        if cfg!(debug_assertions) {
            core::char::from_u32(val).unwrap()
        } else {
            core::char::from_u32_unchecked(val)
        }
    }

    pub unsafe fn bool_lift(val: u8) -> bool {
        if cfg!(debug_assertions) {
            match val {
                0 => false,
                1 => true,
                _ => panic!("invalid bool discriminant"),
            }
        } else {
            core::mem::transmute::<u8, bool>(val)
        }
    }
}
