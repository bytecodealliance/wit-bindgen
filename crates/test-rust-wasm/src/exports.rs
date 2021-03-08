#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::export!("tests/wasm.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::export!({ paths: ["tests/wasm.witx"], unchecked });

use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

struct MyWasm {
    scalar: AtomicU32,
}

fn wasm() -> &'static impl Wasm {
    static ME: MyWasm = MyWasm {
        scalar: AtomicU32::new(0),
    };
    &ME
}

impl Wasm for MyWasm {
    fn allocated_bytes(&self) -> u32 {
        crate::allocator::get() as u32
    }

    fn run_import_tests(&self) {
        crate::imports::run();
    }

    fn roundtrip_u8(&self, a: u8) -> u8 {
        a
    }

    fn roundtrip_s8(&self, a: i8) -> i8 {
        a
    }

    fn roundtrip_u16(&self, a: u16) -> u16 {
        a
    }

    fn roundtrip_s16(&self, a: i16) -> i16 {
        a
    }

    fn roundtrip_u32(&self, a: u32) -> u32 {
        a
    }

    fn roundtrip_s32(&self, a: i32) -> i32 {
        a
    }

    fn roundtrip_u64(&self, a: u64) -> u64 {
        a
    }

    fn roundtrip_s64(&self, a: i64) -> i64 {
        a
    }

    fn roundtrip_f32(&self, a: f32) -> f32 {
        a
    }

    fn roundtrip_f64(&self, a: f64) -> f64 {
        a
    }

    fn roundtrip_char(&self, a: char) -> char {
        a
    }

    fn multiple_results(&self) -> (u8, u16) {
        (100, 200)
    }

    fn set_scalar(&self, val: u32) {
        self.scalar.store(val, SeqCst)
    }

    fn get_scalar(&self) -> u32 {
        self.scalar.load(SeqCst)
    }

    fn swap_tuple(&self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&self, a: F1) -> F1 {
        a
    }

    fn roundtrip_flags2(&self, a: F2) -> F2 {
        a
    }

    fn roundtrip_record1(&self, a: R1) -> R1 {
        a
    }

    fn tuple0(&self, _: ()) {}

    fn tuple1(&self, a: (u8,)) -> (u8,) {
        (a.0,)
    }

    fn roundtrip_option(&self, a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(&self, a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(&self, a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(&self, a: bool) -> bool {
        !a
    }

    fn variant_casts(&self, a: Casts) -> Casts {
        a
    }

    fn variant_zeros(&self, a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(&self, _: Option<u32>, _: bool, _: Result<u32, ()>) {}

    fn list_param(&self, list: Vec<u8>) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(&self, ptr: String) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(&self, ptr: Vec<String>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(&self, ptr: Vec<Vec<String>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_result(&self) -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2(&self) -> String {
        "hello!".to_string()
    }

    fn list_result3(&self) -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }
}
