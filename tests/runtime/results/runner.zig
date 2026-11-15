const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const t = wit.@"test";

comptime {
    wit.exports.run(run);
}

fn run() void {
    assert(std.mem.eql(u8, t.stringError(0.0).err, "zero"));
    assert(t.stringError(1.0).ok == 1.0);

    assert(t.enumError(0.0).err == .a);
    assert(t.enumError(1.0).ok == 1.0);

    assert(std.meta.eql(t.recordError(0.0).err, .{ .line = 420, .column = 0 }));
    assert(std.meta.eql(t.recordError(1.0).err, .{ .line = 77, .column = 2 }));
    assert(t.recordError(2.0).ok == 2.0);

    assert(std.meta.eql(t.variantError(0.0).err, .{ .e2 = .{ .line = 420, .column = 0 } }));
    assert(std.meta.eql(t.variantError(1.0).err, .{ .e1 = .b }));
    assert(std.meta.eql(t.variantError(2.0).err, .{ .e1 = .c }));
    assert(t.variantError(3.0).ok == 3.0);

    assert(t.emptyError(0) == .err);
    assert(t.emptyError(1).ok == 42);
    assert(t.emptyError(2).ok == 2);

    assert(t.doubleError(0).ok == .ok);
    assert(std.mem.eql(u8, t.doubleError(1).ok.err, "one"));
    assert(std.mem.eql(u8, t.doubleError(2).err, "two"));
}
