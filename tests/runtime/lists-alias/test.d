import wit.my.lists.test;
import wit.common;

@witExport("cat", "foo")
void foo(in WitList!ubyte x) {
    assert(x == (cast(immutable ubyte[])"hello").witList);
}

@witExport("cat", "bar")
WitList!ubyte bar() {
    return (cast(immutable ubyte[])"world").witList.witClone;
}

alias Exports = wit.my.lists.test.Exports!(
    foo,
    bar
);
