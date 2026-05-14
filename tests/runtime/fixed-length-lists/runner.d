//@ wasmtime-flags = '-Wcomponent-model-fixed-length-lists'

import wit.test.fixed_length_lists.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    listParam([1, 2, 3, 4]);
    listParam2([[1, 2], [3, 4]]);
    listParam3([
        -1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
    ]);
    {
        auto result = listResult();
        assert(result == ['0', '1', 'A', 'B', 'a', 'b', 128, 255]);
    }
    {
        auto result = listMinmax16([0, 1024, 32768, 65535], [1, 2048, -32767, -2]);
        assert(result == Tuple!(ushort[4], short[4])([0, 1024, 32768, 65535], [1, 2048, -32767, -2]));
    }
    {
        auto result = listMinmaxFloat([2.0, -42.0], [0.25, -0.125]);
        assert(result == Tuple!(float[2], float[2])([2.0, -42.0], [0.25, -0.125]));
    }
    {
        auto result = listRoundtrip(['a', 'b', 'c', 'd', 0, 1, 2, 3, 'A', 'B', 'Y', 'Z']);
        assert(result == ['a', 'b', 'c', 'd', 0, 1, 2, 3, 'A', 'B', 'Y', 'Z']);
    }
    {
        auto result = nestedRoundtrip([[1, 5], [42, 1_000_000]], [[-1, 3], [-2_000_000, 4711]]);
        assert(
            result ==
            Tuple!(uint[2][2], int[2][2])([[1, 5], [42, 1_000_000]], [[-1, 3], [-2_000_000, 4711]])
        );
    }
    {
        auto result = largeRoundtrip(
            [[1, 5], [42, 1_000_000]],
            [
                [-1, 3, -2, 4],
                [-2_000_000, 4711, 99_999, -5],
                [-6, 7, 8, -9],
                [50, -5, 500, -5000],
            ],
        );
        assert(
            result ==
            Tuple!(uint[2][2], int[4][4])(
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
        auto result = nightmareOnCpp([Nested(l: [1, -1]), Nested(l: [2, -2])]);
        assert(result[0].l == [1, -1]);
        assert(result[1].l == [2, -2]);
    }
}

alias Exports = wit.test.fixed_length_lists.runner.Exports!(
    run
);
