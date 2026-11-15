const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.listParam(listParam);
    wit.exports.listParam2(listParam2);
    wit.exports.listParam3(listParam3);
    wit.exports.listResult(listResult);
    wit.exports.listMinmax16(listMinmax16);
    wit.exports.listMinmaxFloat(listMinmaxFloat);
    wit.exports.listRoundtrip(listRoundtrip);
    wit.exports.nestedRoundtrip(nestedRoundtrip);
    wit.exports.largeRoundtrip(largeRoundtrip);
    wit.exports.nightmareOnCpp(nightmareOnCpp);
}

fn listParam(a: [4]u32) void {
    assert(std.meta.eql(a, .{ 1, 2, 3, 4 }));
}

fn listParam2(a: [2][2]u32) void {
    assert(std.meta.eql(a, .{ .{ 1, 2 }, .{ 3, 4 } }));
}

fn listParam3(a: [20]i32) void {
    assert(std.meta.eql(a, .{
        -1,  2,  -3,  4,  -5,  6,  -7,  8,  -9,  10,
        -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
    }));
}

fn listResult() [8]u8 {
    return .{ '0', '1', 'A', 'B', 'a', 'b', 128, 255 };
}

fn listMinmax16(a: [4]u16, b: [4]i16) struct { [4]u16, [4]i16 } {
    return .{ a, b };
}

fn listMinmaxFloat(a: [2]f32, b: [2]f64) struct { [2]f32, [2]f64 } {
    return .{ a, b };
}

fn listRoundtrip(a: [12]u8) [12]u8 {
    return a;
}

fn nestedRoundtrip(a: [2][2]u32, b: [2][2]i32) struct { [2][2]u32, [2][2]i32 } {
    return .{ a, b };
}

fn largeRoundtrip(a: [2][2]u32, b: [4][4]i32) struct { [2][2]u32, [4][4]i32 } {
    return .{ a, b };
}

fn nightmareOnCpp(a: [2]to_test.Nested) [2]to_test.Nested {
    return a;
}
