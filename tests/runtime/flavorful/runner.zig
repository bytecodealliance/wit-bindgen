const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    to_test.fListInRecord1(.{ .a = "list_in_record1" });
    assert(std.mem.eql(u8, to_test.fListInRecord2().a, "list_in_record2"));
    assert(std.mem.eql(u8, to_test.fListInRecord3(.{ .a = "list_in_record3 input" }).a, "list_in_record3 output"));
    assert(std.mem.eql(u8, to_test.fListInRecord4(.{ .a = "input4" }).a, "result4"));

    to_test.fListInVariant1("foo", .{ .err = "bar" });
    assert(std.mem.eql(u8, to_test.fListInVariant2().?, "list_in_variant2"));
    assert(std.mem.eql(u8, to_test.fListInVariant3("input3").?, "output3"));

    assert(to_test.errnoResult().err == .b);
    assert(to_test.errnoResult() == .ok);

    const td = to_test.listTypedefs("typedef1", &.{"typedef2"});
    assert(std.mem.eql(u8, td[0], "typedef3"));
    assert(td[1].len == 1);
    assert(std.mem.eql(u8, td[1][0], "typedef4"));

    const v = to_test.listOfVariants(
        &.{ true, false },
        &.{ .{ .ok = {} }, .{ .err = {} } },
        &.{ .success, .a },
    );
    assert(std.mem.eql(bool, v[0], &.{ false, true }));
    assert(v[1].len == 2 and v[1][0] == .err and v[1][1] == .ok);
    assert(std.mem.eql(to_test.MyErrno, v[2], &.{ .a, .b }));
}
