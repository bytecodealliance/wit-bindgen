import wit.test.strings.test;

import wit.common;


@witInterface("test:strings/to-test") {
    @witExport
    void takeBasic(in WitString str) {
        assert(str == "latin utf16");
    }
    
    @witExport
    WitString returnUnicode() {
        return "🚀🚀🚀 𠈄𓀀".witList.witClone;
    }
    
    @witExport
    WitString returnEmpty() {
        return WitString();
    }
    
    @witExport
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
