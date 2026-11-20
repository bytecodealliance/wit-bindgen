const wit = @import("wit");

comptime {
    wit.exports.run(run);
}

fn run() void {
    wit.the_test.x();
}
