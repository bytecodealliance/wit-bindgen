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
    {
        let result = list_minmax16([0, 1024, 32768, 65535], [1, 2048, -32767, -2]);
        assert_eq!(result, ([0, 1024, 32768, 65535], [1, 2048, -32767, -2]));
    }
    {
        let result = list_minmax_float([2.0, -42.0], [0.25, -0.125]);
        assert_eq!(result, ([2.0, -42.0], [0.25, -0.125]));
    }
    {
        let result = list_roundtrip([b'a', b'b', b'c', b'd', 0, 1, 2, 3, b'A', b'B', b'Y', b'Z']);
        assert_eq!(
            result,
            [b'a', b'b', b'c', b'd', 0, 1, 2, 3, b'A', b'B', b'Y', b'Z']
        );
    }
    {
        let result = nested_roundtrip([[1, 5], [42, 1_000_000]], [[-1, 3], [-2_000_000, 4711]]);
        assert_eq!(
            result,
            ([[1, 5], [42, 1_000_000]], [[-1, 3], [-2_000_000, 4711]])
        );
    }
    {
        let result = large_roundtrip(
            [[1, 5], [42, 1_000_000]],
            [
                [-1, 3, -2, 4],
                [-2_000_000, 4711, 99_999, -5],
                [-6, 7, 8, -9],
                [50, -5, 500, -5000],
            ],
        );
        assert_eq!(
            result,
            (
                [[1, 5], [42, 1_000_000]],
                [
                    [-1, 3, -2, 4],
                    [-2_000_000, 4711, 99_999, -5],
                    [-6, 7, 8, -9],
                    [50, -5, 500, -5000]
                ]
            )
        );
    }
    {
        let result = nightmare_on_cpp([Nested { l: [1, -1] }, Nested { l: [2, -2] }]);
        assert_eq!(result[0].l, [1, -1]);
        assert_eq!(result[1].l, [2, -2]);
    }
}
