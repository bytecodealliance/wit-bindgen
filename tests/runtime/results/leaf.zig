const wit = @import("wit");
const t = wit.@"test";

comptime {
    wit.exports.stringError(stringError);
    wit.exports.enumError(enumError);
    wit.exports.recordError(recordError);
    wit.exports.variantError(variantError);
    wit.exports.emptyError(emptyError);
    wit.exports.doubleError(doubleError);
}

fn stringError(a: f32) wit.abi.Result(f32, []const u8) {
    if (a == 0.0) return .{ .err = "zero" };
    return .{ .ok = a };
}

fn enumError(a: f32) wit.abi.Result(f32, t.E) {
    if (a == 0.0) return .{ .err = .a };
    return .{ .ok = a };
}

fn recordError(a: f32) wit.abi.Result(f32, t.E2) {
    if (a == 0.0) return .{ .err = .{ .line = 420, .column = 0 } };
    if (a == 1.0) return .{ .err = .{ .line = 77, .column = 2 } };
    return .{ .ok = a };
}

fn variantError(a: f32) wit.abi.Result(f32, t.E3) {
    if (a == 0.0) return .{ .err = .{ .e2 = .{ .line = 420, .column = 0 } } };
    if (a == 1.0) return .{ .err = .{ .e1 = .b } };
    if (a == 2.0) return .{ .err = .{ .e1 = .c } };
    return .{ .ok = a };
}

fn emptyError(a: u32) wit.abi.Result(u32, void) {
    if (a == 0) return .{ .err = {} };
    if (a == 1) return .{ .ok = 42 };
    return .{ .ok = a };
}

fn doubleError(a: u32) wit.abi.Result(wit.abi.Result(void, []const u8), []const u8) {
    if (a == 0) return .{ .ok = .{ .ok = {} } };
    if (a == 1) return .{ .ok = .{ .err = "one" } };
    return .{ .err = "two" };
}
