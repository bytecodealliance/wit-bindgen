import wit.my.inline.test;

import wit.common;

@witExport("my:inline/foo1", "foo")
void foo1() {}

@witExport("my:inline/foo2", "foo")
void foo2() {}

@witExport("my:inline/bar1", "bar")
WitString bar1() { return WitString(); }

@witExport("my:inline/bar2", "bar")
WitString bar2() { return WitString(); }


alias Exports = wit.my.inline.test.Exports!(
    foo1,
    foo2,
    bar1,
    bar2
);
