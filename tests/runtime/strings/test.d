import wit.test.strings.test;

import wit.common;


@witInterface("test:strings/to-test") {
    @witExport("take-basic")
    void takeBasic(in WitString str) {
        assert(str == "latin utf16");
    }
    
    @witExport("return-unicode")
    WitString returnUnicode() {
        return "🚀🚀🚀 𠈄𓀀".witList.witClone;
    }
    
    @witExport("return-empty")
    WitString returnEmpty() {
        return WitString();
    }
    
    @witExport("roundtrip")
    WitString roundtrip(in WitString str) {
        return str.witClone;
    }
}

alias Exports = wit.test.strings.test.Exports!(
    takeBasic,
    returnUnicode,
    returnEmpty,
    roundtrip
);
