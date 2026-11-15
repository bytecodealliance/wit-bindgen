const wit = @import("wit");
const to_test = wit.to_test;

comptime {
    wit.exports.multipleResults(multipleResults);
    wit.exports.swapTuple(swapTuple);
    wit.exports.roundtripFlags1(roundtripFlags1);
    wit.exports.roundtripFlags2(roundtripFlags2);
    wit.exports.roundtripFlags3(roundtripFlags3);
    wit.exports.roundtripRecord1(roundtripRecord1);
    wit.exports.tuple1(tuple1);
}

fn multipleResults() struct { u8, u16 } {
    return .{ 4, 5 };
}

fn swapTuple(a: struct { u8, u32 }) struct { u32, u8 } {
    return .{ a[1], a[0] };
}

fn roundtripFlags1(a: to_test.F1) to_test.F1 {
    return a;
}

fn roundtripFlags2(a: to_test.F2) to_test.F2 {
    return a;
}

fn roundtripFlags3(a: to_test.Flag8, b: to_test.Flag16, c: to_test.Flag32) struct { to_test.Flag8, to_test.Flag16, to_test.Flag32 } {
    return .{ a, b, c };
}

fn roundtripRecord1(a: to_test.R1) to_test.R1 {
    return a;
}

fn tuple1(a: struct { u8 }) struct { u8 } {
    return .{a[0]};
}
