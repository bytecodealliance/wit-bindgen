const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    assert(to_test.roundtripOption(1.0).? == 1);
    assert(to_test.roundtripOption(null) == null);
    assert(to_test.roundtripOption(2.0).? == 2);
    assert(to_test.roundtripOption(4.0).? == 4);
    assert(to_test.roundtripOption(5.3).? == 5);

    assert(std.meta.eql(to_test.roundtripResult(.{ .ok = 2 }), .{ .ok = 2.0 }));
    assert(std.meta.eql(to_test.roundtripResult(.{ .ok = 4 }), .{ .ok = 4.0 }));
    assert(std.meta.eql(to_test.roundtripResult(.{ .err = 5.3 }), .{ .err = 5 }));

    assert(to_test.roundtripEnum(.a) == .a);
    assert(to_test.roundtripEnum(.b) == .b);

    assert(to_test.invertBool(true) == false);
    assert(to_test.invertBool(false) == true);

    {
        const input: to_test.Casts = .{
            .{ .a = 1 },
            .{ .a = 2 },
            .{ .a = 3 },
            .{ .a = 4 },
            .{ .a = 5 },
            .{ .a = 6.0 },
        };
        assert(std.meta.eql(to_test.variantCasts(input), input));
    }

    {
        const input: to_test.Casts = .{
            .{ .b = 1 },
            .{ .b = 2.0 },
            .{ .b = 3.0 },
            .{ .b = 4.0 },
            .{ .b = 5.0 },
            .{ .b = 6.0 },
        };
        assert(std.meta.eql(to_test.variantCasts(input), input));
    }

    {
        const input: to_test.Zeros = .{
            .{ .a = 1 },
            .{ .a = 2 },
            .{ .a = 3.0 },
            .{ .a = 4.0 },
        };
        assert(std.meta.eql(to_test.variantZeros(input), input));
    }

    {
        const input: to_test.Zeros = .{
            .b,
            .b,
            .b,
            .b,
        };
        assert(std.meta.eql(to_test.variantZeros(input), input));
    }

    to_test.variantTypedefs(null, false, .{ .err = {} });

    const enums = to_test.variantEnums(true, .{ .ok = {} }, .success);
    assert(enums[0] == true);
    assert(std.meta.eql(enums[1], .{ .ok = {} }));
    assert(enums[2] == .success);
}
