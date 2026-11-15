const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.run(run);
}

fn run() void {
    const thing = to_test.Thing.init(42);
    assert(to_test.foo(thing) == 42 + 1 + 2);
}
