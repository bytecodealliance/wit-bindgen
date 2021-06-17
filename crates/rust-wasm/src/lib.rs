pub use witx_bindgen_rust_impl::{export, import};

pub mod exports;
pub mod imports;

#[doc(hidden)]
pub mod rt {
    use std::alloc::{self, Layout};

    #[no_mangle]
    unsafe extern "C" fn canonical_abi_realloc(
        old_ptr: *mut u8,
        old_len: usize,
        len: usize,
        align: usize,
    ) -> *mut u8 {
        let layout;
        let ptr = if old_len == 0 {
            layout = Layout::from_size_align_unchecked(len, align);
            alloc::alloc(layout)
        } else {
            layout = Layout::from_size_align_unchecked(old_len, align);
            alloc::realloc(old_ptr, layout, len)
        };
        if ptr.is_null() {
            alloc::handle_alloc_error(layout);
        }
        return ptr;
    }

    #[no_mangle]
    unsafe extern "C" fn canonical_abi_free(ptr: *mut u8, len: usize, align: usize) {
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
