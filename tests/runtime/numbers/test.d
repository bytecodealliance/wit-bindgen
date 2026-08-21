import wit.test.numbers.test;
import wit.common;

template roundtrip(T, string suffix) {
    @witExport("test:numbers/numbers", "roundtrip-"~suffix)
    T roundtrip(T val) => val;
}

uint scalar;

@witExport("test:numbers/numbers", "get-scalar")
auto getScalar() => scalar;

@witExport("test:numbers/numbers", "set-scalar")
void setScalar(uint val) { scalar = val; }

alias Exports = wit.test.numbers.test.Exports!(
    roundtrip!(ubyte, "u8"),
    roundtrip!(byte, "s8"),
    roundtrip!(ushort, "u16"),
    roundtrip!(short, "s16"),
    roundtrip!(uint, "u32"),
    roundtrip!(int, "s32"),
    roundtrip!(ulong, "u64"),
    roundtrip!(long, "s64"),
    roundtrip!(float, "f32"),
    roundtrip!(double, "f64"),
    roundtrip!(dchar, "char"),

    getScalar,
    setScalar
);
