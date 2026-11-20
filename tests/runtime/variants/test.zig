const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.roundtripOption(roundtripOption);
    wit.exports.roundtripResult(roundtripResult);
    wit.exports.roundtripEnum(roundtripEnum);
    wit.exports.invertBool(invertBool);
    wit.exports.variantCasts(variantCasts);
    wit.exports.variantZeros(variantZeros);
    wit.exports.variantTypedefs(variantTypedefs);
    wit.exports.variantEnums(variantEnums);
}

fn roundtripOption(a: ?f32) ?u8 {
    if (a) |v| return @intFromFloat(v);
    return null;
}

fn roundtripResult(a: wit.abi.Result(u32, f32)) wit.abi.Result(f64, u8) {
    return switch (a) {
        .ok => |v| .{ .ok = @floatFromInt(v) },
        .err => |e| .{ .err = @intFromFloat(e) },
    };
}

fn roundtripEnum(a: to_test.E1) to_test.E1 {
    return a;
}

fn invertBool(a: bool) bool {
    return !a;
}

fn variantCasts(a: to_test.Casts) to_test.Casts {
    return a;
}

fn variantZeros(a: to_test.Zeros) to_test.Zeros {
    return a;
}

fn variantTypedefs(_: to_test.OptionTypedef, _: to_test.BoolTypedef, _: to_test.ResultTypedef) void {}

fn variantEnums(a: bool, b: wit.abi.Result(void, void), c: to_test.MyErrno) struct { bool, wit.abi.Result(void, void), to_test.MyErrno } {
    return .{ a, b, c };
}
