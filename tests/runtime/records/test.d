import wit.test.records.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witInterface("test:records/to-test") {
    @witExport
    Tuple!(ubyte, ushort) multipleResults() {
        return tuple(ubyte(4), ushort(5));
    }
    
    @witExport
    Tuple!(uint, ubyte) swapTuple(in Tuple!(ubyte, uint) a) {
        return tuple(a[1], a[0]);
    }
    
    @witExport
    F1 roundtripFlags1(F1 a) {
        return a;
    }
    
    @witExport
    F2 roundtripFlags2(F2 a) {
        return a;
    }
    
    @witExport
    Tuple!(Flag8, Flag16, Flag32) roundtripFlags3(Flag8 a, Flag16 b, Flag32 c) {
        return tuple(a, b, c);
    }
    
    @witExport
    R1 roundtripRecord1(in R1 a) {
        return a;
    }
    
    @witExport
    Tuple!(ubyte) tuple1(in Tuple!(ubyte) a) {
        return tuple(a[0]);
    }
}

alias Exports = wit.test.records.test.Exports!(
    multipleResults,
    swapTuple,
    roundtripFlags1,
    roundtripFlags2,
    roundtripFlags3,
    roundtripRecord1,
    tuple1
);
