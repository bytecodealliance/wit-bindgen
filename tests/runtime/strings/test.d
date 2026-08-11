import wit.test.strings.test;

import wit.common;

@witExport("test:strings/to-test", "take-basic")
void takeBasic(in WitString str) {
    assert(str == "latin utf16");
}

@witExport("test:strings/to-test", "return-unicode")
WitString returnUnicode() {
    return "🚀🚀🚀 𠈄𓀀".witList.witClone;
}

@witExport("test:strings/to-test", "return-empty")
WitString returnEmpty() {
    return WitString();
}

@witExport("test:strings/to-test", "roundtrip")
WitString roundtrip(in WitString str) {
    return str.witClone;
}


alias Exports = wit.test.strings.test.Exports!(
    takeBasic,
    returnUnicode,
    returnEmpty,
    roundtrip
);
