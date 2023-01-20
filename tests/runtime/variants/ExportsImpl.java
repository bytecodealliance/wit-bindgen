package wit_variants;

import wit_variants.VariantsWorld.Result;
import wit_variants.VariantsWorld.Tuple0;
import wit_variants.VariantsWorld.Tuple3;
import wit_variants.VariantsWorld.Tuple4;
import wit_variants.VariantsWorld.Tuple6;

public class ExportsImpl {
    public static Byte roundtripOption(Float a) {
        return a == null ? null : (byte) (float) a;
    }

    public static Result<Double, Byte> roundtripResult(Result<Integer, Float> a) {
        switch (a.tag) {
        case Result.OK: return Result.ok((double) a.getOk());
        case Result.ERR: return Result.err((byte) (float) a.getErr());
        default: throw new AssertionError();
        }
    }

    public static Exports.E1 roundtripEnum(Exports.E1 a) {
        return a;
    }

    public static boolean invertBool(boolean a) {
        return !a;
    }

    public static Tuple6<Exports.C1, Exports.C2, Exports.C3, Exports.C4, Exports.C5, Exports.C6>
        variantCasts(Tuple6<Exports.C1, Exports.C2, Exports.C3, Exports.C4, Exports.C5, Exports.C6> a)
    {
        return a;
    }

    public static Tuple4<Exports.Z1, Exports.Z2, Exports.Z3, Exports.Z4>
        variantZeros(Tuple4<Exports.Z1, Exports.Z2, Exports.Z3, Exports.Z4> a)
    {
        return a;
    }

    public static void variantTypedefs(Integer a, boolean b, Result<Integer, Tuple0> c) { }

    public static Tuple3<Boolean, Result<Tuple0, Tuple0>, Exports.MyErrno>
        variantEnums(boolean a, Result<Tuple0, Tuple0> b, Exports.MyErrno c)
    {
        return new Tuple3<>(a, b, c);
    }
}
