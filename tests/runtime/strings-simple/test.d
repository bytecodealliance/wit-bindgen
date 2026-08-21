import wit.my.strings.test;

import wit.common;

@witExport("cat", "foo")
void foo(in WitString str) {
    assert(str == "hello");
}

@witExport("cat", "bar")
WitString bar() {
    return "world".witList.witClone;
}

alias Exports = wit.my.strings.test.Exports!(
    foo,
    bar
);
