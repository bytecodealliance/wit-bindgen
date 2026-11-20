const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

var first: bool = true;

comptime {
    wit.exports.fListInRecord1(fListInRecord1);
    wit.exports.fListInRecord2(fListInRecord2);
    wit.exports.fListInRecord3(fListInRecord3);
    wit.exports.fListInRecord4(fListInRecord4);
    wit.exports.fListInVariant1(fListInVariant1);
    wit.exports.fListInVariant2(fListInVariant2);
    wit.exports.fListInVariant3(fListInVariant3);
    wit.exports.errnoResult(errnoResult);
    wit.exports.listTypedefs(listTypedefs);
    wit.exports.listOfVariants(listOfVariants);
}

fn fListInRecord1(a: to_test.ListInRecord1) void {
    assert(std.mem.eql(u8, a.a, "list_in_record1"));
}

fn fListInRecord2() to_test.ListInRecord2 {
    return .{ .a = "list_in_record2" };
}

fn fListInRecord3(a: to_test.ListInRecord3) to_test.ListInRecord3 {
    assert(std.mem.eql(u8, a.a, "list_in_record3 input"));
    return .{ .a = "list_in_record3 output" };
}

fn fListInRecord4(a: to_test.ListInAlias) to_test.ListInAlias {
    assert(std.mem.eql(u8, a.a, "input4"));
    return .{ .a = "result4" };
}

fn fListInVariant1(a: to_test.ListInVariant1V1, b: to_test.ListInVariant1V2) void {
    assert(std.mem.eql(u8, a.?, "foo"));
    assert(std.mem.eql(u8, b.err, "bar"));
}

fn fListInVariant2() to_test.ListInVariant2 {
    return "list_in_variant2";
}

fn fListInVariant3(a: to_test.ListInVariant3) to_test.ListInVariant3 {
    assert(std.mem.eql(u8, a.?, "input3"));
    return "output3";
}

fn errnoResult() wit.abi.Result(void, to_test.MyErrno) {
    if (first) {
        first = false;
        return .{ .err = .b };
    }
    return .{ .ok = {} };
}

fn listTypedefs(a: to_test.ListTypedef, c: to_test.ListTypedef3) struct { to_test.ListTypedef2, to_test.ListTypedef3 } {
    assert(std.mem.eql(u8, a, "typedef1"));
    assert(c.len == 1);
    assert(std.mem.eql(u8, c[0], "typedef2"));
    return .{ "typedef3", &.{"typedef4"} };
}

fn listOfVariants(
    a: []const bool,
    b: []const wit.abi.Result(void, void),
    c: []const to_test.MyErrno,
) struct { []const bool, []const wit.abi.Result(void, void), []const to_test.MyErrno } {
    assert(std.mem.eql(bool, a, &.{ true, false }));
    assert(b.len == 2 and b[0] == .ok and b[1] == .err);
    assert(std.mem.eql(to_test.MyErrno, c, &.{ .success, .a }));
    return .{
        &.{ false, true },
        &.{ .{ .err = {} }, .{ .ok = {} } },
        &.{ .a, .b },
    };
}
