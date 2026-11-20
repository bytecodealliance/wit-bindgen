const wit = @import("wit");

comptime {
    wit.exports.run(run);
}

fn run() void {
    wit.to_test.manyArguments(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
}
