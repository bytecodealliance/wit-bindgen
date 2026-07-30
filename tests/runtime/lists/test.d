import wit.test.lists.test;
import wit.common;

@witExport("test:lists/to-test", "empty-list-param")
void emptyListParam(in WitList!ubyte a) {
}

@witExport("test:lists/to-test", "empty-string-param")
void emptyStringParam(in WitString a) {
}

@witExport("test:lists/to-test", "empty-list-result")
WitList!ubyte emptyListResult() {
    return WitList!ubyte();
}

@witExport("test:lists/to-test", "empty-string-result")
WitString emptyStringResult() {
    return WitString();
}

@witExport("test:lists/to-test", "list-param")
void listParam(in WitList!ubyte a) {
}

@witExport("test:lists/to-test", "list-param2")
void listParam2(in WitString a) {
}

@witExport("test:lists/to-test", "list-param3")
void listParam3(in WitList!WitString a) {
}

@witExport("test:lists/to-test", "list-param4")
void listParam4(in WitList!(WitList!WitString) a) {
}

@witExport("test:lists/to-test", "list-param5")
void listParam5(in WitList!(Tuple!(ubyte, uint, ubyte)) a) {
}

@witExport("test:lists/to-test", "list-param-large")
void listParamLarge(in WitList!WitString a) {
}

@witExport("test:lists/to-test", "list-result")
WitList!ubyte listResult() {
    immutable ubyte[5] outputs = [1, 2, 3, 4, 5];
    return outputs.witList.witClone;
}

@witExport("test:lists/to-test", "list-result2")
WitString listResult2() {
    return "hello!".witList.witClone;
}

@witExport("test:lists/to-test", "list-result3")
WitList!WitString listResult3() {
    immutable WitString[2] outputs = ["hello,".witList, "world!".witList];
    return outputs.witList.witClone;
}

template listMinmax(T, U, string suffix) {
    @witExport("test:lists/to-test", "list-minmax"~suffix)
    Tuple!(WitList!T, WitList!U) listMinmax(in WitList!T a, in WitList!U b) {
        return tuple(a, b).witClone;
    }
}

@witExport("test:lists/to-test", "list-roundtrip")
WitList!ubyte listRoundtrip(in WitList!ubyte a) {
    return a.witClone;
}

@witExport("test:lists/to-test", "string-roundtrip")
WitString stringRoundtrip(in WitString a) {
    return a.witClone;
}

@witExport("test:lists/to-test", "wasi-http-headers-roundtrip")
WitList!(Tuple!(WitString, WitList!ubyte)) wasiHttpHeadersRoundtrip(in WitList!(Tuple!(WitString, WitList!ubyte)) a) {
    return a.witClone;
}


extern extern(C) size_t walloc_allocated_bytes;
@witExport("test:lists/to-test", "allocated-bytes")
size_t allocatedBytes() {
    return walloc_allocated_bytes;
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
