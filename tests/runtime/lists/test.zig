const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");

comptime {
    wit.exports.emptyListParam(emptyListParam);
    wit.exports.emptyStringParam(emptyStringParam);
    wit.exports.emptyListResult(emptyListResult);
    wit.exports.emptyStringResult(emptyStringResult);
    wit.exports.listParam(listParam);
    wit.exports.listParam2(listParam2);
    wit.exports.listParam3(listParam3);
    wit.exports.listParam4(listParam4);
    wit.exports.listParam5(listParam5);
    wit.exports.listParamLarge(listParamLarge);
    wit.exports.listResult(listResult);
    wit.exports.listResult2(listResult2);
    wit.exports.listResult3(listResult3);
    wit.exports.listMinmax8(listMinmax8);
    wit.exports.listMinmax16(listMinmax16);
    wit.exports.listMinmax32(listMinmax32);
    wit.exports.listMinmax64(listMinmax64);
    wit.exports.listMinmaxFloat(listMinmaxFloat);
    wit.exports.listRoundtrip(listRoundtrip);
    wit.exports.stringRoundtrip(stringRoundtrip);
    wit.exports.wasiHttpHeadersRoundtrip(wasiHttpHeadersRoundtrip);
    wit.exports.allocatedBytes(allocatedBytes);
}

fn emptyListParam(a: []const u8) void {
    assert(a.len == 0);
}

fn emptyStringParam(a: []const u8) void {
    assert(a.len == 0);
}

fn emptyListResult() []const u8 {
    return &.{};
}

fn emptyStringResult() []const u8 {
    return "";
}

fn listParam(a: []const u8) void {
    assert(std.mem.eql(u8, a, &.{ 1, 2, 3, 4 }));
}

fn listParam2(a: []const u8) void {
    assert(std.mem.eql(u8, a, "foo"));
}

fn listParam3(a: []const []const u8) void {
    assert(a.len == 3);
    assert(std.mem.eql(u8, a[0], "foo"));
    assert(std.mem.eql(u8, a[1], "bar"));
    assert(std.mem.eql(u8, a[2], "baz"));
}

fn listParam4(a: []const []const []const u8) void {
    assert(a.len == 2);
    assert(a[0].len == 2);
    assert(a[1].len == 1);
    assert(std.mem.eql(u8, a[0][0], "foo"));
    assert(std.mem.eql(u8, a[0][1], "bar"));
    assert(std.mem.eql(u8, a[1][0], "baz"));
}

fn listParam5(a: []const struct { u8, u32, u8 }) void {
    assert(a.len == 2);
    assert(a[0][0] == 1 and a[0][1] == 2 and a[0][2] == 3);
    assert(a[1][0] == 4 and a[1][1] == 5 and a[1][2] == 6);
}

fn listParamLarge(a: []const []const u8) void {
    assert(a.len == 1000);
}

fn listResult() []const u8 {
    return &.{ 1, 2, 3, 4, 5 };
}

fn listResult2() []const u8 {
    return "hello!";
}

fn listResult3() []const []const u8 {
    return &.{ "hello,", "world!" };
}

fn listMinmax8(a: []const u8, b: []const i8) struct { []const u8, []const i8 } {
    return .{ a, b };
}

fn listMinmax16(a: []const u16, b: []const i16) struct { []const u16, []const i16 } {
    return .{ a, b };
}

fn listMinmax32(a: []const u32, b: []const i32) struct { []const u32, []const i32 } {
    return .{ a, b };
}

fn listMinmax64(a: []const u64, b: []const i64) struct { []const u64, []const i64 } {
    return .{ a, b };
}

fn listMinmaxFloat(a: []const f32, b: []const f64) struct { []const f32, []const f64 } {
    return .{ a, b };
}

fn listRoundtrip(a: []const u8) []const u8 {
    return a;
}

fn stringRoundtrip(a: []const u8) []const u8 {
    return a;
}

fn wasiHttpHeadersRoundtrip(a: []const struct { []const u8, []const u8 }) []const struct { []const u8, []const u8 } {
    return a;
}

fn allocatedBytes() u32 {
    return 0;
}
