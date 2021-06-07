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
        let layout = Layout::from_size_align_unchecked(len, align);
        alloc::dealloc(ptr, layout);
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

    as_i32!(char i8 u8 i16 u16 i32 u32 usize);

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
