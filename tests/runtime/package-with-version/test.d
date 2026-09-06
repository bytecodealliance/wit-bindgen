import wit.my.inline.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witInterface("my:inline/foo@0.0.0")
@witExport("bar")
struct BarImpl {
    @witInterface("my:inline/foo@0.0.0")
    @witExport("[constructor]bar")
    static Bar constructor() {
        return Bar.makeNew((out typeof(this) self) {
        });
    }
}

alias Exports = wit.my.inline.test.Exports!(
    BarImpl
);
