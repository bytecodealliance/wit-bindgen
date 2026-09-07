import wit.test.variants.test;

import wit.common;

@witInterface("test:variants/to-test") {
    @witExport
    Option!ubyte roundtripOption(in Option!float a) {
        if (a.isSome) return (cast(ubyte)a.unwrap).some;
        return none!ubyte;
    }
    
    @witExport
    Result!(double, ubyte) roundtripResult(in Result!(uint, float) a) {
        if (a.isOk) return (cast(double)a.unwrap).ok!ubyte;
        return (cast(ubyte)a.unwrapErr).err!double;
    }
    
    @witExport
    E1 roundtripEnum(E1 a) => a;
    
    @witExport
    bool invertBool(bool a) => !a;
    
    @witExport
    Casts variantCasts(in Casts a) => a;
    
    @witExport
    Zeros variantZeros(in Zeros a) => a;
    
    @witExport
    void variantTypedefs(in Option!uint, bool, in Result!uint) {}
    
    
    @witExport
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
