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
    fn list_result() -> [u8; 8] {
        [b'0', b'1', b'A', b'B', b'a', b'b', 128, 255]
    }
}
