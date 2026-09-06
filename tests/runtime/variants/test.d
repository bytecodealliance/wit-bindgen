import wit.test.variants.test;

import wit.common;

@witInterface("test:variants/to-test") {
    @witExport("roundtrip-option")
    Option!ubyte roundtripOption(in Option!float a) {
        if (a.isSome) return (cast(ubyte)a.unwrap).some;
        return none!ubyte;
    }
    
    @witExport("roundtrip-result")
    Result!(double, ubyte) roundtripResult(in Result!(uint, float) a) {
        if (a.isOk) return (cast(double)a.unwrap).ok!ubyte;
        return (cast(ubyte)a.unwrapErr).err!double;
    }
    
    @witExport("roundtrip-enum")
    E1 roundtripEnum(E1 a) => a;
    
    @witExport("invert-bool")
    bool invertBool(bool a) => !a;
    
    @witExport("variant-casts")
    Casts variantCasts(in Casts a) => a;
    
    @witExport("variant-zeros")
    Zeros variantZeros(in Zeros a) => a;
    
    @witExport("variant-typedefs")
    void variantTypedefs(in Option!uint, bool, in Result!uint) {}
    
    
    @witExport("variant-enums")
    Tuple!(bool, Result!void, MyErrno) variantEnums(bool a, in Result!void b, MyErrno c) {
        return tuple(a, b, c);
    }
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
