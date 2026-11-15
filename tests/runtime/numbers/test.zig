const wit = @import("wit");

var scalar: u32 = 0;

comptime {
    wit.exports.roundtripU8(roundtripU8);
    wit.exports.roundtripS8(roundtripS8);
    wit.exports.roundtripU16(roundtripU16);
    wit.exports.roundtripS16(roundtripS16);
    wit.exports.roundtripU32(roundtripU32);
    wit.exports.roundtripS32(roundtripS32);
    wit.exports.roundtripU64(roundtripU64);
    wit.exports.roundtripS64(roundtripS64);
    wit.exports.roundtripF32(roundtripF32);
    wit.exports.roundtripF64(roundtripF64);
    wit.exports.roundtripChar(roundtripChar);
    wit.exports.setScalar(setScalar);
    wit.exports.getScalar(getScalar);
}

fn roundtripU8(a: u8) u8 {
    return a;
}

fn roundtripS8(a: i8) i8 {
    return a;
}

fn roundtripU16(a: u16) u16 {
    return a;
}

fn roundtripS16(a: i16) i16 {
    return a;
}

fn roundtripU32(a: u32) u32 {
    return a;
}

fn roundtripS32(a: i32) i32 {
    return a;
}

fn roundtripU64(a: u64) u64 {
    return a;
}

fn roundtripS64(a: i64) i64 {
    return a;
}

fn roundtripF32(a: f32) f32 {
    return a;
}

fn roundtripF64(a: f64) f64 {
    return a;
}

fn roundtripChar(a: u21) u21 {
    return a;
}

fn setScalar(a: u32) void {
    scalar = a;
}

fn getScalar() u32 {
    return scalar;
}
