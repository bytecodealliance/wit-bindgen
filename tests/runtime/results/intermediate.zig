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
    return t.stringError(a);
}

fn enumError(a: f32) wit.abi.Result(f32, t.E) {
    return t.enumError(a);
}

fn recordError(a: f32) wit.abi.Result(f32, t.E2) {
    return t.recordError(a);
}

fn variantError(a: f32) wit.abi.Result(f32, t.E3) {
    return t.variantError(a);
}

fn emptyError(a: u32) wit.abi.Result(u32, void) {
    return t.emptyError(a);
}

fn doubleError(a: u32) wit.abi.Result(wit.abi.Result(void, []const u8), []const u8) {
    return t.doubleError(a);
}
