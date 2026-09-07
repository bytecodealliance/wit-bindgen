import wit.test.fixed_length_lists.test;
import wit.common;

@witInterface("test:fixed-length-lists/to-test") {
    @witExport
    void listParam(in uint[4] a) {
        assert(a == [1, 2, 3, 4]);
    }
    
    @witExport
    void listParam2(in uint[2][2] a) {
        enum uint[2][2] v = [[1, 2], [3, 4]];
        assert(a == v);
    }
    
    @witExport
    void listParam3(in int[20] a) {
        assert(a == [-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20]);
    }
    
    @witExport
    Tuple!(ushort[4], short[4]) listMinmax16(in ushort[4] a, in short[4] b) {
        return tuple(a, b);
    }
    
    
    @witExport
    Tuple!(float[2], double[2]) listMinmaxFloat(in float[2] a, in double[2] b) {
        return tuple(a, b);
    }
    
    @witExport
    ubyte[12] listRoundtrip(in ubyte[12] a) => a;
    
    @witExport
    ubyte[8] listResult() => ['0', '1', 'A', 'B', 'a', 'b', 128, 255];
    
    @witExport
    Tuple!(uint[2][2], int[2][2]) nestedRoundtrip(in uint[2][2] a, in int[2][2] b) {
        return tuple(a, b);
    }
    
    @witExport
    Tuple!(uint[2][2], int[4][4]) largeRoundtrip(in uint[2][2] a, in int[4][4] b) {
        return tuple(a, b);
    }
    
    @witExport
    Nested[2] nightmareOnCPP(in Nested[2] a) {
        return a;
    }
}

alias Exports = wit.test.fixed_length_lists.test.Exports!(
    listParam,
    listParam2,
    listParam3,
    listMinmax16,
    listMinmaxFloat,
    listRoundtrip,
    listResult,
    nestedRoundtrip,
    largeRoundtrip,
    nightmareOnCPP
);
