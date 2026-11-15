const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const numbers = wit.numbers;

comptime {
    wit.exports.run(run);
}

fn run() void {
    assert(numbers.roundtripU8(1) == 1);
    assert(numbers.roundtripU8(0) == 0);
    assert(numbers.roundtripU8(std.math.maxInt(u8)) == std.math.maxInt(u8));

    assert(numbers.roundtripS8(1) == 1);
    assert(numbers.roundtripS8(std.math.minInt(i8)) == std.math.minInt(i8));
    assert(numbers.roundtripS8(std.math.maxInt(i8)) == std.math.maxInt(i8));

    assert(numbers.roundtripU16(1) == 1);
    assert(numbers.roundtripU16(0) == 0);
    assert(numbers.roundtripU16(std.math.maxInt(u16)) == std.math.maxInt(u16));

    assert(numbers.roundtripS16(1) == 1);
    assert(numbers.roundtripS16(std.math.minInt(i16)) == std.math.minInt(i16));
    assert(numbers.roundtripS16(std.math.maxInt(i16)) == std.math.maxInt(i16));

    assert(numbers.roundtripU32(1) == 1);
    assert(numbers.roundtripU32(0) == 0);
    assert(numbers.roundtripU32(std.math.maxInt(u32)) == std.math.maxInt(u32));

    assert(numbers.roundtripS32(1) == 1);
    assert(numbers.roundtripS32(std.math.minInt(i32)) == std.math.minInt(i32));
    assert(numbers.roundtripS32(std.math.maxInt(i32)) == std.math.maxInt(i32));

    assert(numbers.roundtripU64(1) == 1);
    assert(numbers.roundtripU64(0) == 0);
    assert(numbers.roundtripU64(std.math.maxInt(u64)) == std.math.maxInt(u64));

    assert(numbers.roundtripS64(1) == 1);
    assert(numbers.roundtripS64(std.math.minInt(i64)) == std.math.minInt(i64));
    assert(numbers.roundtripS64(std.math.maxInt(i64)) == std.math.maxInt(i64));

    assert(numbers.roundtripF32(1.0) == 1.0);
    assert(numbers.roundtripF32(std.math.inf(f32)) == std.math.inf(f32));
    assert(numbers.roundtripF32(-std.math.inf(f32)) == -std.math.inf(f32));
    assert(std.math.isNan(numbers.roundtripF32(std.math.nan(f32))));

    assert(numbers.roundtripF64(1.0) == 1.0);
    assert(numbers.roundtripF64(std.math.inf(f64)) == std.math.inf(f64));
    assert(numbers.roundtripF64(-std.math.inf(f64)) == -std.math.inf(f64));
    assert(std.math.isNan(numbers.roundtripF64(std.math.nan(f64))));

    assert(numbers.roundtripChar('a') == 'a');
    assert(numbers.roundtripChar(' ') == ' ');
    assert(numbers.roundtripChar('🚩') == '🚩');

    numbers.setScalar(2);
    assert(numbers.getScalar() == 2);
    numbers.setScalar(4);
    assert(numbers.getScalar() == 4);
}
