const wit = @import("wit");

comptime {
    wit.exports.x(x);
}

fn x() void {}
