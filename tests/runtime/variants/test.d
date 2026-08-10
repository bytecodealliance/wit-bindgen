import wit.test.variants.test;

import wit.common;

@witExport("test:variants/to-test", "roundtrip-option")
Option!ubyte roundtripOption(in Option!float a) {
    if (a.isSome) return (cast(ubyte)a.unwrap).some;
    return none!ubyte;
}

@witExport("test:variants/to-test", "roundtrip-result")
Result!(double, ubyte) roundtripResult(in Result!(uint, float) a) {
    if (a.isOk) return (cast(double)a.unwrap).ok!ubyte;
    return (cast(ubyte)a.unwrapErr).err!double;
}

@witExport("test:variants/to-test", "roundtrip-enum")
E1 roundtripEnum(E1 a) => a;

@witExport("test:variants/to-test", "invert-bool")
bool invertBool(bool a) => !a;

@witExport("test:variants/to-test", "variant-casts")
Casts variantCasts(in Casts a) => a;

@witExport("test:variants/to-test", "variant-zeros")
Zeros variantZeros(in Zeros a) => a;

@witExport("test:variants/to-test", "variant-typedefs")
void variantTypedefs(in Option!uint, bool, in Result!uint) {}


@witExport("test:variants/to-test", "variant-enums")
Tuple!(bool, Result!void, MyErrno) variantEnums(bool a, in Result!void b, MyErrno c) {
    return tuple(a, b, c);
}

alias Exports = wit.test.variants.test.Exports!(
    roundtripOption,
    roundtripResult,
    roundtripEnum,
    invertBool,
    variantCasts,
    variantZeros,
    variantTypedefs,
    variantEnums
);
