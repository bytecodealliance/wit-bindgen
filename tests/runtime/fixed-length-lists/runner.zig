//@ wasmtime-flags = '-Wcomponent-model-fixed-length-lists'

const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    to_test.listParam(.{ 1, 2, 3, 4 });
    to_test.listParam2(.{ .{ 1, 2 }, .{ 3, 4 } });
    to_test.listParam3(.{
        -1,  2,  -3,  4,  -5,  6,  -7,  8,  -9,  10,
        -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
    });
    assert(std.meta.eql(to_test.listResult(), .{ '0', '1', 'A', 'B', 'a', 'b', 128, 255 }));
    assert(std.meta.eql(
        to_test.listMinmax16(.{ 0, 1024, 32768, 65535 }, .{ 1, 2048, -32767, -2 }),
        .{ .{ 0, 1024, 32768, 65535 }, .{ 1, 2048, -32767, -2 } },
    ));
    assert(std.meta.eql(
        to_test.listMinmaxFloat(.{ 2.0, -42.0 }, .{ 0.25, -0.125 }),
        .{ .{ 2.0, -42.0 }, .{ 0.25, -0.125 } },
    ));
    assert(std.meta.eql(
        to_test.listRoundtrip(.{ 'a', 'b', 'c', 'd', 0, 1, 2, 3, 'A', 'B', 'Y', 'Z' }),
        .{ 'a', 'b', 'c', 'd', 0, 1, 2, 3, 'A', 'B', 'Y', 'Z' },
    ));
    assert(std.meta.eql(
        to_test.nestedRoundtrip(.{ .{ 1, 5 }, .{ 42, 1_000_000 } }, .{ .{ -1, 3 }, .{ -2_000_000, 4711 } }),
        .{ .{ .{ 1, 5 }, .{ 42, 1_000_000 } }, .{ .{ -1, 3 }, .{ -2_000_000, 4711 } } },
    ));
    const b: [4][4]i32 = .{
        .{ -1, 3, -2, 4 },
        .{ -2_000_000, 4711, 99_999, -5 },
        .{ -6, 7, 8, -9 },
        .{ 50, -5, 500, -5000 },
    };
    assert(std.meta.eql(
        to_test.largeRoundtrip(.{ .{ 1, 5 }, .{ 42, 1_000_000 } }, b),
        .{ .{ .{ 1, 5 }, .{ 42, 1_000_000 } }, b },
    ));
    const nested: [2]to_test.Nested = .{ .{ .l = .{ 1, -1 } }, .{ .l = .{ 2, -2 } } };
    assert(std.meta.eql(to_test.nightmareOnCpp(nested), nested));
}
