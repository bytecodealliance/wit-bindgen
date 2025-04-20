include!(env!("BINDINGS"));

struct Component;

export!(Component);

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
}
