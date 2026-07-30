import wit.test.many_arguments.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witExport("test:many-arguments/to-test", "many-arguments")
void manyArguments(Repeat!(16, ulong) args) {
    assert(args == AliasSeq!(
        1,  2,  3,  4,  5,  6,  7,  8,
        9,  10, 11, 12, 13, 14, 15, 16
    ));
}

alias Exports = wit.test.many_arguments.test.Exports!(
    manyArguments
);
