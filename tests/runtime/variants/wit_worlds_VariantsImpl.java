package wit.worlds;

import wit.worlds.Variants.Result;
import wit.worlds.Variants.Tuple0;
import wit.worlds.Variants.Tuple3;
import wit.worlds.Variants.Tuple4;
import wit.worlds.Variants.Tuple6;
import wit.imports.test.variants.Test;

public class VariantsImpl {
    public static void testImports() {
        expect(Test.roundtripOption(1.0F) == (byte) 1);
        expect(Test.roundtripOption(null) == null);
        expect(Test.roundtripOption(2.0F) == (byte) 2);

        {
            Result<Double, Byte> result = Test.roundtripResult(Result.ok(2));
            expect(result.tag == Result.OK && result.getOk() == 2.0D);
        }

        {
            Result<Double, Byte> result = Test.roundtripResult(Result.ok(4));
            expect(result.tag == Result.OK && result.getOk() == 4.0D);
        }

        {
            Result<Double, Byte> result = Test.roundtripResult(Result.err(5.3F));
            expect(result.tag == Result.ERR && result.getErr() == (byte) 5);
        }


        expect(Test.roundtripEnum(Test.E1.A) == Test.E1.A);
        expect(Test.roundtripEnum(Test.E1.B) == Test.E1.B);

        expect(Test.invertBool(true) == false);
        expect(Test.invertBool(false) == true);

        {
            Tuple6<Test.C1, Test.C2, Test.C3, Test.C4, Test.C5, Test.C6> result
                = Test.variantCasts(new Tuple6<>(Test.C1.a(1),
                                                 Test.C2.a(2),
                                                 Test.C3.a(3),
                                                 Test.C4.a(4L),
                                                 Test.C5.a(5L),
                                                 Test.C6.a(6.0F)));

            expect(result.f0.tag == Test.C1.A && result.f0.getA() == 1);
            expect(result.f1.tag == Test.C2.A && result.f1.getA() == 2);
            expect(result.f2.tag == Test.C3.A && result.f2.getA() == 3);
            expect(result.f3.tag == Test.C4.A && result.f3.getA() == 4L);
            expect(result.f4.tag == Test.C5.A && result.f4.getA() == 5L);
            expect(result.f5.tag == Test.C6.A && result.f5.getA() == 6.0F);
        }

        {
            Tuple6<Test.C1, Test.C2, Test.C3, Test.C4, Test.C5, Test.C6> result
                = Test.variantCasts(new Tuple6<>(Test.C1.b(1L),
                                                 Test.C2.b(2.0F),
                                                 Test.C3.b(3.0D),
                                                 Test.C4.b(4.0F),
                                                 Test.C5.b(5.0D),
                                                 Test.C6.b(6.0D)));

            expect(result.f0.tag == Test.C1.B && result.f0.getB() == 1L);
            expect(result.f1.tag == Test.C2.B && result.f1.getB() == 2.0F);
            expect(result.f2.tag == Test.C3.B && result.f2.getB() == 3.0D);
            expect(result.f3.tag == Test.C4.B && result.f3.getB() == 4.0F);
            expect(result.f4.tag == Test.C5.B && result.f4.getB() == 5.0D);
            expect(result.f5.tag == Test.C6.B && result.f5.getB() == 6.0D);
        }

        {
            Tuple4<Test.Z1, Test.Z2, Test.Z3, Test.Z4> result
                = Test.variantZeros(new Tuple4<>(Test.Z1.a(1),
                                                 Test.Z2.a(2L),
                                                 Test.Z3.a(3.0F),
                                                 Test.Z4.a(4.0D)));

            expect(result.f0.tag == Test.Z1.A && result.f0.getA() == 1);
            expect(result.f1.tag == Test.Z2.A && result.f1.getA() == 2L);
            expect(result.f2.tag == Test.Z3.A && result.f2.getA() == 3.0F);
            expect(result.f3.tag == Test.Z4.A && result.f3.getA() == 4.0D);
        }

        {
            Tuple4<Test.Z1, Test.Z2, Test.Z3, Test.Z4> result
                = Test.variantZeros(new Tuple4<>(Test.Z1.b(),
                                                 Test.Z2.b(),
                                                 Test.Z3.b(),
                                                 Test.Z4.b()));

            expect(result.f0.tag == Test.Z1.B);
            expect(result.f1.tag == Test.Z2.B);
            expect(result.f2.tag == Test.Z3.B);
            expect(result.f3.tag == Test.Z4.B);
        }

        Test.variantTypedefs(null, false, Result.err(Tuple0.INSTANCE));

        {
            Tuple3<Boolean, Result<Tuple0, Tuple0>, Test.MyErrno> result
                = Test.variantEnums(true, Result.ok(Tuple0.INSTANCE), Test.MyErrno.SUCCESS);

            expect(result.f0 == false);
            expect(result.f1.tag == Result.ERR);
            expect(result.f2 == Test.MyErrno.A);
        }
    }

    private static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }
}
