const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    to_test.takeBasic("latin utf16");
    assert(std.mem.eql(u8, to_test.returnUnicode(), "🚀🚀🚀 𠈄𓀀"));
    assert(std.mem.eql(u8, to_test.returnEmpty(), ""));
    assert(std.mem.eql(u8, to_test.roundtrip("🚀🚀🚀 𠈄𓀀"), "🚀🚀🚀 𠈄𓀀"));
}
