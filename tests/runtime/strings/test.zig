const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");

comptime {
    wit.exports.takeBasic(takeBasic);
    wit.exports.returnUnicode(returnUnicode);
    wit.exports.returnEmpty(returnEmpty);
    wit.exports.roundtrip(roundtrip);
}

fn takeBasic(s: []const u8) void {
    assert(std.mem.eql(u8, s, "latin utf16"));
}

fn returnUnicode() []const u8 {
    return "🚀🚀🚀 𠈄𓀀";
}

fn returnEmpty() []const u8 {
    return "";
}

fn roundtrip(s: []const u8) []const u8 {
    assert(s.len > 0);
    return s;
}
