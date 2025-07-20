include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::test::fixed_size_lists::to_test::Nested;

impl exports::test::fixed_size_lists::to_test::Guest for Component {
    fn list_param(a: [u32; 4]) {
        assert_eq!(a, [1, 2, 3, 4]);
    }
    fn list_param2(a: [[u32; 2]; 2]) {
        assert_eq!(a, [[1, 2], [3, 4]]);
    }
    fn list_param3(a: [i32; 20]) {
        assert_eq!(
            a,
            [-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20]
        );
    }
    fn list_minmax16(a: [u16; 4], b: [i16; 4]) -> ([u16; 4], [i16; 4]) {
        (a, b)
    }
    fn list_minmax_float(a: [f32; 2], b: [f64; 2]) -> ([f32; 2], [f64; 2]) {
        (a, b)
    }
    fn list_roundtrip(a: [u8; 12]) -> [u8; 12] {
        a
    }
    fn list_result() -> [u8; 8] {
        [b'0', b'1', b'A', b'B', b'a', b'b', 128, 255]
    }
    fn nested_roundtrip(a: [[u32; 2]; 2], b: [[i32; 2]; 2]) -> ([[u32; 2]; 2], [[i32; 2]; 2]) {
        (a, b)
    }
    fn large_roundtrip(a: [[u32; 2]; 2], b: [[i32; 4]; 4]) -> ([[u32; 2]; 2], [[i32; 4]; 4]) {
        (a, b)
    }
    fn nightmare_on_cpp(a: [Nested; 2]) -> [Nested; 2] {
        a
    }
}
