import wit.my.strings.test;

import wit.common;

@witInterface("cat") {
    @witExport("foo")
    void foo(in WitString str) {
        assert(str == "hello");
    }
    
    @witExport("bar")
    WitString bar() {
        return "world".witList.witClone;
    }
}

alias Exports = wit.my.strings.test.Exports!(
    foo,
    bar
);
