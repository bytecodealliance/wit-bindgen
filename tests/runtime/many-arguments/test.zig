const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");

comptime {
    wit.exports.manyArguments(manyArguments);
}

fn manyArguments(
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    a8: u64,
    a9: u64,
    a10: u64,
    a11: u64,
    a12: u64,
    a13: u64,
    a14: u64,
    a15: u64,
    a16: u64,
) void {
    assert(a1 == 1);
    assert(a2 == 2);
    assert(a3 == 3);
    assert(a4 == 4);
    assert(a5 == 5);
    assert(a6 == 6);
    assert(a7 == 7);
    assert(a8 == 8);
    assert(a9 == 9);
    assert(a10 == 10);
    assert(a11 == 11);
    assert(a12 == 12);
    assert(a13 == 13);
    assert(a14 == 14);
    assert(a15 == 15);
    assert(a16 == 16);
}
