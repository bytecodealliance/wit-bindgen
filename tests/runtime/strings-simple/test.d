import wit.my.strings.test;

import wit.common;

@witInterface("cat") {
    @witExport
    void foo(in WitString str) {
        assert(str == "hello");
    }
    
    @witExport
    WitString bar() {
        return "world".witList.witClone;
    }
}

alias Exports = wit.my.strings.test.Exports!(
    foo,
    bar
);
