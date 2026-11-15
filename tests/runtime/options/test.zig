const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");

comptime {
    wit.exports.optionNoneParam(optionNoneParam);
    wit.exports.optionSomeParam(optionSomeParam);
    wit.exports.optionNoneResult(optionNoneResult);
    wit.exports.optionSomeResult(optionSomeResult);
    wit.exports.optionRoundtrip(optionRoundtrip);
    wit.exports.doubleOptionRoundtrip(doubleOptionRoundtrip);
}

fn optionNoneParam(a: ?[]const u8) void {
    assert(a == null);
}

fn optionSomeParam(a: ?[]const u8) void {
    assert(std.mem.eql(u8, a.?, "foo"));
}

fn optionNoneResult() ?[]const u8 {
    return null;
}

fn optionSomeResult() ?[]const u8 {
    return "foo";
}

fn optionRoundtrip(a: ?[]const u8) ?[]const u8 {
    return a;
}

fn doubleOptionRoundtrip(a: ??u32) ??u32 {
    return a;
}
