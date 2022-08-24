wit_bindgen_guest_rust::import!("../../tests/runtime/lists/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/lists/exports.wit");

use std::alloc::{self, Layout};
use std::mem;
use std::ptr;

struct Exports;

impl exports::Exports for Exports {
    fn allocated_bytes() -> u32 {
        test_rust_wasm::get() as u32
    }

    fn test_imports() {
        use imports::*;

        let _guard = test_rust_wasm::guard();

        empty_list_param(&[]);
        empty_string_param("");
        assert!(empty_list_result().is_empty());
        assert!(empty_string_result().is_empty());

        list_param(&[1, 2, 3, 4]);
        list_param2("foo");
        list_param3(&["foo", "bar", "baz"]);
        list_param4(&[&["foo", "bar"], &["baz"]]);
        assert_eq!(list_result(), [1, 2, 3, 4, 5]);
        assert_eq!(list_result2(), "hello!");
        assert_eq!(list_result3(), ["hello,", "world!"]);

        assert_eq!(string_roundtrip("x"), "x");
        assert_eq!(string_roundtrip(""), "");
        assert_eq!(string_roundtrip("hello"), "hello");
        assert_eq!(string_roundtrip("hello ⚑ world"), "hello ⚑ world");

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

            fn as_slice(&self) -> *const [T] {
                unsafe { ptr::slice_from_raw_parts(self.alloc.add(1).cast(), 1) }
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
            // Technically this is UB because we're creating safe slices from
            // unaligned pointers, but we're hoping that because we're just passing
            // off pointers to an import through a safe import we can get away with
            // this. If this ever becomes a problem we'll just need to call the raw
            // import with raw integers.
            unaligned_roundtrip1(
                &*u16s.as_slice(),
                &*u32s.as_slice(),
                &*u64s.as_slice(),
                &*flag32s.as_slice(),
                &*flag64s.as_slice(),
            );
            unaligned_roundtrip2(
                &*records.as_slice(),
                &*f32s.as_slice(),
                &*f64s.as_slice(),
                &*strings.as_slice(),
                &*lists.as_slice(),
            );
        }

        assert_eq!(
            list_minmax8(&[u8::MIN, u8::MAX], &[i8::MIN, i8::MAX]),
            (vec![u8::MIN, u8::MAX], vec![i8::MIN, i8::MAX]),
        );
        assert_eq!(
            list_minmax16(&[u16::MIN, u16::MAX], &[i16::MIN, i16::MAX]),
            (vec![u16::MIN, u16::MAX], vec![i16::MIN, i16::MAX]),
        );
        assert_eq!(
            list_minmax32(&[u32::MIN, u32::MAX], &[i32::MIN, i32::MAX]),
            (vec![u32::MIN, u32::MAX], vec![i32::MIN, i32::MAX]),
        );
        assert_eq!(
            list_minmax64(&[u64::MIN, u64::MAX], &[i64::MIN, i64::MAX]),
            (vec![u64::MIN, u64::MAX], vec![i64::MIN, i64::MAX]),
        );
        assert_eq!(
            list_minmax_float(
                &[f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
                &[f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]
            ),
            (
                vec![f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
                vec![f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY],
            ),
        );
    }

    fn empty_list_param(a: Vec<u8>) {
        assert!(a.is_empty());
    }

    fn empty_string_param(a: String) {
        assert!(a.is_empty());
    }

    fn empty_list_result() -> Vec<u8> {
        Vec::new()
    }

    fn empty_string_result() -> String {
        String::new()
    }

    fn list_param(list: Vec<u8>) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(ptr: String) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(ptr: Vec<String>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(ptr: Vec<Vec<String>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_result() -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2() -> String {
        "hello!".to_string()
    }

    fn list_result3() -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }

    fn list_roundtrip(x: Vec<u8>) -> Vec<u8> {
        x.clone()
    }

    fn string_roundtrip(x: String) -> String {
        x.clone()
    }
}
