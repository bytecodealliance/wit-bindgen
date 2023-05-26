package wit.exports.test.variants;

import wit.worlds.Variants.Result;
import wit.worlds.Variants.Tuple0;
import wit.worlds.Variants.Tuple3;
import wit.worlds.Variants.Tuple4;
import wit.worlds.Variants.Tuple6;

public class TestImpl {
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

    public static Test.E1 roundtripEnum(Test.E1 a) {
        return a;
    }

    public static boolean invertBool(boolean a) {
        return !a;
    }

    public static Tuple6<Test.C1, Test.C2, Test.C3, Test.C4, Test.C5, Test.C6>
        variantCasts(Tuple6<Test.C1, Test.C2, Test.C3, Test.C4, Test.C5, Test.C6> a)
    {
        return a;
    }

    public static Tuple4<Test.Z1, Test.Z2, Test.Z3, Test.Z4>
        variantZeros(Tuple4<Test.Z1, Test.Z2, Test.Z3, Test.Z4> a)
    {
        return a;
    }

    public static void variantTypedefs(Integer a, boolean b, Result<Integer, Tuple0> c) { }

    public static Tuple3<Boolean, Result<Tuple0, Tuple0>, Test.MyErrno>
        variantEnums(boolean a, Result<Tuple0, Tuple0> b, Test.MyErrno c)
    {
        return new Tuple3<>(a, b, c);
    }
}
