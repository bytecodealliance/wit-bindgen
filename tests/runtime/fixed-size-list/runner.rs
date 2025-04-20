include!(env!("BINDINGS"));

use test::fixed_size_lists::to_test::*;

fn main() {
    list_param([1, 2, 3, 4]);
    list_param2([[1, 2], [3, 4]]);
    list_param3([
        -1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
    ]);
    {
        let result = list_result();
        assert_eq!(result, [b'0', b'1', b'A', b'B', b'a', b'b', 128, 255]);
    }
}
