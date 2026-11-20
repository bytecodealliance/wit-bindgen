const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    const res = to_test.multipleResults();
    assert(res[0] == 4 and res[1] == 5);

    const swapped = to_test.swapTuple(.{ 1, 2 });
    assert(swapped[0] == 2 and swapped[1] == 1);

    assert(std.meta.eql(to_test.roundtripFlags1(.{ .a = true }), .{ .a = true }));
    assert(std.meta.eql(to_test.roundtripFlags1(.{}), .{}));
    assert(std.meta.eql(to_test.roundtripFlags1(.{ .b = true }), .{ .b = true }));
    assert(std.meta.eql(to_test.roundtripFlags1(.{ .a = true, .b = true }), .{ .a = true, .b = true }));

    assert(std.meta.eql(to_test.roundtripFlags2(.{ .c = true }), .{ .c = true }));
    assert(std.meta.eql(to_test.roundtripFlags2(.{}), .{}));
    assert(std.meta.eql(to_test.roundtripFlags2(.{ .d = true }), .{ .d = true }));
    assert(std.meta.eql(to_test.roundtripFlags2(.{ .c = true, .e = true }), .{ .c = true, .e = true }));

    const f3 = to_test.roundtripFlags3(.{ .b0 = true }, .{ .b1 = true }, .{ .b2 = true });
    assert(std.meta.eql(f3[0], .{ .b0 = true }));
    assert(std.meta.eql(f3[1], .{ .b1 = true }));
    assert(std.meta.eql(f3[2], .{ .b2 = true }));

    const r1 = to_test.roundtripRecord1(.{ .a = 8, .b = .{} });
    assert(r1.a == 8);
    assert(std.meta.eql(r1.b, .{}));

    const r2 = to_test.roundtripRecord1(.{ .a = 0, .b = .{ .a = true, .b = true } });
    assert(r2.a == 0);
    assert(std.meta.eql(r2.b, .{ .a = true, .b = true }));

    assert(to_test.tuple1(.{1})[0] == 1);
}
