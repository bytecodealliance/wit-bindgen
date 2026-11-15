const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    const before = to_test.allocatedBytes();
    defer assert(to_test.allocatedBytes() == before);

    to_test.emptyListParam(&.{});
    to_test.emptyStringParam("");
    assert(to_test.emptyListResult().len == 0);
    assert(to_test.emptyStringResult().len == 0);

    to_test.listParam(&.{ 1, 2, 3, 4 });
    to_test.listParam2("foo");
    to_test.listParam3(&.{ "foo", "bar", "baz" });
    to_test.listParam4(&.{ &.{ "foo", "bar" }, &.{"baz"} });
    to_test.listParam5(&.{ .{ 1, 2, 3 }, .{ 4, 5, 6 } });

    const large = wit.abi.arena().alloc([]const u8, 1000) catch @panic("OOM");
    @memset(large, "string");
    to_test.listParamLarge(large);

    assert(std.mem.eql(u8, to_test.listResult(), &.{ 1, 2, 3, 4, 5 }));
    assert(std.mem.eql(u8, to_test.listResult2(), "hello!"));
    const r3 = to_test.listResult3();
    assert(r3.len == 2);
    assert(std.mem.eql(u8, r3[0], "hello,"));
    assert(std.mem.eql(u8, r3[1], "world!"));

    assert(std.mem.eql(u8, to_test.listRoundtrip(&.{}), &.{}));
    assert(std.mem.eql(u8, to_test.listRoundtrip("x"), "x"));
    assert(std.mem.eql(u8, to_test.listRoundtrip("hello"), "hello"));

    assert(std.mem.eql(u8, to_test.stringRoundtrip("x"), "x"));
    assert(std.mem.eql(u8, to_test.stringRoundtrip(""), ""));
    assert(std.mem.eql(u8, to_test.stringRoundtrip("hello"), "hello"));
    assert(std.mem.eql(u8, to_test.stringRoundtrip("hello ⚑ world"), "hello ⚑ world"));

    const m8 = to_test.listMinmax8(&.{ 0, std.math.maxInt(u8) }, &.{ std.math.minInt(i8), std.math.maxInt(i8) });
    assert(std.mem.eql(u8, m8[0], &.{ 0, std.math.maxInt(u8) }));
    assert(std.mem.eql(i8, m8[1], &.{ std.math.minInt(i8), std.math.maxInt(i8) }));

    const m16 = to_test.listMinmax16(&.{ 0, std.math.maxInt(u16) }, &.{ std.math.minInt(i16), std.math.maxInt(i16) });
    assert(std.mem.eql(u16, m16[0], &.{ 0, std.math.maxInt(u16) }));
    assert(std.mem.eql(i16, m16[1], &.{ std.math.minInt(i16), std.math.maxInt(i16) }));

    const m32 = to_test.listMinmax32(&.{ 0, std.math.maxInt(u32) }, &.{ std.math.minInt(i32), std.math.maxInt(i32) });
    assert(std.mem.eql(u32, m32[0], &.{ 0, std.math.maxInt(u32) }));
    assert(std.mem.eql(i32, m32[1], &.{ std.math.minInt(i32), std.math.maxInt(i32) }));

    const m64 = to_test.listMinmax64(&.{ 0, std.math.maxInt(u64) }, &.{ std.math.minInt(i64), std.math.maxInt(i64) });
    assert(std.mem.eql(u64, m64[0], &.{ 0, std.math.maxInt(u64) }));
    assert(std.mem.eql(i64, m64[1], &.{ std.math.minInt(i64), std.math.maxInt(i64) }));

    const mf = to_test.listMinmaxFloat(&.{ -std.math.inf(f32), std.math.inf(f32) }, &.{ -std.math.inf(f64), std.math.inf(f64) });
    assert(std.mem.eql(f32, mf[0], &.{ -std.math.inf(f32), std.math.inf(f32) }));
    assert(std.mem.eql(f64, mf[1], &.{ -std.math.inf(f64), std.math.inf(f64) }));

    const headers = to_test.wasiHttpHeadersRoundtrip(&.{
        .{ "Content-Type", "text/plain" },
        .{ "Content-Length", "Not found" },
    });
    assert(headers.len == 2);
    assert(std.mem.eql(u8, headers[0][0], "Content-Type"));
    assert(std.mem.eql(u8, headers[0][1], "text/plain"));
    assert(std.mem.eql(u8, headers[1][0], "Content-Length"));
    assert(std.mem.eql(u8, headers[1][1], "Not found"));
}
