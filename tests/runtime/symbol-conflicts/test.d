import wit.my.inline.test;

import wit.common;

@witInterface("my:inline/foo1")
@witExport("foo")
void foo1() {}

@witInterface("my:inline/foo2")
@witExport("foo")
void foo2() {}

@witInterface("my:inline/bar1")
@witExport("bar")
WitString bar1() { return WitString(); }

@witInterface("my:inline/bar2")
@witExport("bar")
WitString bar2() { return WitString(); }


alias Exports = wit.my.inline.test.Exports!(
    foo1,
    foo2,
    bar1,
    bar2
);
