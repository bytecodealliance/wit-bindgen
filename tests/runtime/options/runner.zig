const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    to_test.optionNoneParam(null);
    to_test.optionSomeParam("foo");
    assert(to_test.optionNoneResult() == null);
    assert(std.mem.eql(u8, to_test.optionSomeResult().?, "foo"));
    assert(std.mem.eql(u8, to_test.optionRoundtrip("foo").?, "foo"));
    assert(to_test.optionRoundtrip(null) == null);

    assert(to_test.doubleOptionRoundtrip(42).?.? == 42);
    assert(to_test.doubleOptionRoundtrip(@as(?u32, null)).? == null);
    assert(to_test.doubleOptionRoundtrip(null) == null);
}
