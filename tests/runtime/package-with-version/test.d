import wit.my.inline.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witExport("my:inline/foo@0.0.0", "bar")
struct BarImpl {
    @witExport("my:inline/foo@0.0.0", "[constructor]bar")
    static Bar constructor() {
        return Bar.makeNew((out typeof(this) self) {
        });
    }
}

alias Exports = wit.my.inline.test.Exports!(
    BarImpl
);
