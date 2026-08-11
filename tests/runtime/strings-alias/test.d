import wit.my.strings.test;

import wit.common;

@witExport("cat", "foo")
void foo(in MyString str) {
    assert(str == "hello");
}

@witExport("cat", "bar")
MyString bar() {
    return "world".witList.witClone;
}

alias Exports = wit.my.strings.test.Exports!(
    foo,
    bar
);
