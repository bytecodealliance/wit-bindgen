import wit.test.lists.test;
import wit.common;

@witInterface("test:lists/to-test") {
    @witExport
    void emptyListParam(in WitList!ubyte a) {
    }
    
    @witExport
    void emptyStringParam(in WitString a) {
    }
    
    @witExport
    WitList!ubyte emptyListResult() {
        return WitList!ubyte();
    }
    
    @witExport
    WitString emptyStringResult() {
        return WitString();
    }
    
    @witExport
    void listParam(in WitList!ubyte a) {
    }
    
    @witExport
    void listParam2(in WitString a) {
    }
    
    @witExport
    void listParam3(in WitList!WitString a) {
    }
    
    @witExport
    void listParam4(in WitList!(WitList!WitString) a) {
    }
    
    @witExport
    void listParam5(in WitList!(Tuple!(ubyte, uint, ubyte)) a) {
    }
    
    @witExport
    void listParamLarge(in WitList!WitString a) {
    }
    
    @witExport
    WitList!ubyte listResult() {
        immutable ubyte[5] outputs = [1, 2, 3, 4, 5];
        return outputs.witList.witClone;
    }
    
    @witExport
    WitString listResult2() {
        return "hello!".witList.witClone;
    }
    
    @witExport
    WitList!WitString listResult3() {
        immutable WitString[2] outputs = ["hello,".witList, "world!".witList];
        return outputs.witList.witClone;
    }
    
    template listMinmax(T, U, string suffix) {
        @witExport("list-minmax"~suffix)
        Tuple!(WitList!T, WitList!U) listMinmax(in WitList!T a, in WitList!U b) {
            return tuple(a, b).witClone;
        }
    }
    
    @witExport
    WitList!ubyte listRoundtrip(in WitList!ubyte a) {
        return a.witClone;
    }
    
    @witExport
    WitString stringRoundtrip(in WitString a) {
        return a.witClone;
    }
    
    @witExport
    WitList!(Tuple!(WitString, WitList!ubyte)) wasiHttpHeadersRoundtrip(in WitList!(Tuple!(WitString, WitList!ubyte)) a) {
        return a.witClone;
    }
    
    
    extern extern(C) size_t walloc_allocated_bytes;
    @witExport
    size_t allocatedBytes() {
        return walloc_allocated_bytes;
    }
}

alias Exports = wit.test.lists.test.Exports!(
    emptyListParam,
    emptyStringParam,
    emptyListResult,
    emptyStringResult,

    listParam,
    listParam2,
    listParam3,
    listParam4,
    listParam5,
    listParamLarge,
    listResult,
    listResult2,
    listResult3,

    listMinmax!(ubyte,  byte,   "8"),
    listMinmax!(ushort, short,  "16"),
    listMinmax!(uint,   int,    "32"),
    listMinmax!(ulong,  long,   "64"),
    listMinmax!(float,  double, "-float"),

    listRoundtrip,

    stringRoundtrip,

    wasiHttpHeadersRoundtrip,

    allocatedBytes
);
