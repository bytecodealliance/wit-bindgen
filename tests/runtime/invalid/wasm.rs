wit_bindgen_guest_rust::export!("../../tests/runtime/invalid/exports.wit");
wit_bindgen_guest_rust::import!("../../tests/runtime/invalid/imports.wit");

#[link(wasm_import_module = "imports")]
extern "C" {
    #[link_name = "roundtrip-bool"]
    fn roundtrip_bool(a: i32) -> i32;
    #[link_name = "roundtrip-u16"]
    fn roundtrip_u16(a: i32) -> i32;
    #[link_name = "roundtrip-u8"]
    fn roundtrip_u8(a: i32) -> i32;
    #[link_name = "roundtrip-s16"]
    fn roundtrip_s16(a: i32) -> i32;
    #[link_name = "roundtrip-s8"]
    fn roundtrip_s8(a: i32) -> i32;
    #[link_name = "roundtrip-char"]
    fn roundtrip_char(a: i32) -> i32;
    #[link_name = "roundtrip-enum"]
    fn roundtrip_enum(a: i32) -> i32;
    #[allow(improper_ctypes)]
    #[link_name = "unaligned-roundtrip1"]
    fn unaligned_roundtrip1(
        _: *const u16,
        _: usize,
        _: *const u32,
        _: usize,
        _: *const u64,
        _: usize,
        _: *const imports::Flag32,
        _: usize,
        _: *const imports::Flag64,
        _: usize,
    );
    #[allow(improper_ctypes)]
    #[link_name = "unaligned-roundtrip2"]
    fn unaligned_roundtrip2(
        _: *const imports::UnalignedRecord,
        _: usize,
        _: *const f32,
        _: usize,
        _: *const f64,
        _: usize,
        _: *const &str,
        _: usize,
        _: *const &[u64],
        _: usize,
    );
}

struct Exports;

impl exports::Exports for Exports {
    fn invalid_bool() {
        unsafe {
            let b = roundtrip_bool(2);
            assert_eq!(b, 1);
        }
    }
    fn invalid_u8() {
        unsafe {
            let u = roundtrip_u8(i32::MAX);
            assert!(u <= (u8::MAX as i32));
            assert!(u >= (u8::MIN as i32));
        }
    }
    fn invalid_s8() {
        unsafe {
            let s = roundtrip_s8(i32::MAX);
            assert!(s <= (i8::MAX as i32));
            assert!(s >= (i8::MIN as i32));
        }
    }
    fn invalid_u16() {
        unsafe {
            let u = roundtrip_u16(i32::MAX);
            assert!(u <= (u16::MAX as i32));
            assert!(u >= (u16::MIN as i32));
        }
    }
    fn invalid_s16() {
        unsafe {
            let s = roundtrip_s16(i32::MAX);
            assert!(s <= (i16::MAX as i32));
            assert!(s >= (i16::MIN as i32));
        }
    }
    fn invalid_char() {
        unsafe {
            roundtrip_char(0xd800);
        }
        unreachable!();
    }
    fn invalid_enum() {
        unsafe {
            roundtrip_enum(400);
        }
        unreachable!();
    }

    fn test_unaligned() {
        use imports::{Flag32, Flag64, UnalignedRecord};
        use std::alloc::{self, Layout};
        use std::mem;
        use std::ptr;

        struct Unaligned<T: Copy> {
            alloc: *mut u8,
            _marker: std::marker::PhantomData<T>,
        }

        impl<T: Copy> Unaligned<T> {
            fn layout() -> Layout {
                Layout::from_size_align(2 * mem::size_of::<T>(), 8).unwrap()
            }

            fn new(data: T) -> Unaligned<T> {
                unsafe {
                    let alloc = alloc::alloc(Self::layout());
                    assert!(!alloc.is_null());
                    ptr::write_unaligned(alloc.add(1).cast(), data);
                    Unaligned {
                        alloc,
                        _marker: Default::default(),
                    }
                }
            }

            fn as_abi(&self) -> (*const T, usize) {
                unsafe { (self.alloc.add(1).cast(), 1) }
            }
        }

        impl<T: Copy> Drop for Unaligned<T> {
            fn drop(&mut self) {
                unsafe {
                    alloc::dealloc(self.alloc, Self::layout());
                }
            }
        }

        unsafe {
            let u16s = Unaligned::new(1);
            let u32s = Unaligned::new(2);
            let u64s = Unaligned::new(3);
            let flag32s = Unaligned::new(Flag32::B8);
            let flag64s = Unaligned::new(Flag64::B9);
            let records = Unaligned::new(UnalignedRecord { a: 10, b: 11 });
            let f32s = Unaligned::new(100.0);
            let f64s = Unaligned::new(101.0);
            let strings = Unaligned::new("foo");
            let lists = Unaligned::new(&[102][..]);

            unaligned_roundtrip1(
                u16s.as_abi().0,
                u16s.as_abi().1,
                u32s.as_abi().0,
                u32s.as_abi().1,
                u64s.as_abi().0,
                u64s.as_abi().1,
                flag32s.as_abi().0,
                flag32s.as_abi().1,
                flag64s.as_abi().0,
                flag64s.as_abi().1,
            );
            unaligned_roundtrip2(
                records.as_abi().0,
                records.as_abi().1,
                f32s.as_abi().0,
                f32s.as_abi().1,
                f64s.as_abi().0,
                f64s.as_abi().1,
                strings.as_abi().0,
                strings.as_abi().1,
                lists.as_abi().0,
                lists.as_abi().1,
            );
        }
    }
}
