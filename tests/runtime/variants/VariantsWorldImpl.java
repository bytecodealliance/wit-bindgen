package wit_variants;

import wit_variants.VariantsWorld.Result;
import wit_variants.VariantsWorld.Tuple0;
import wit_variants.VariantsWorld.Tuple3;
import wit_variants.VariantsWorld.Tuple4;
import wit_variants.VariantsWorld.Tuple6;

public class VariantsWorldImpl {
    public static void testImports() {
        expect(Imports.roundtripOption(1.0F) == (byte) 1);
        expect(Imports.roundtripOption(null) == null);
        expect(Imports.roundtripOption(2.0F) == (byte) 2);

        {
            Result<Double, Byte> result = Imports.roundtripResult(Result.ok(2));
            expect(result.tag == Result.OK && result.getOk() == 2.0D);
        }

        {
            Result<Double, Byte> result = Imports.roundtripResult(Result.ok(4));
            expect(result.tag == Result.OK && result.getOk() == 4.0D);
        }

        {
            Result<Double, Byte> result = Imports.roundtripResult(Result.err(5.3F));
            expect(result.tag == Result.ERR && result.getErr() == (byte) 5);
        }


        expect(Imports.roundtripEnum(Imports.E1.A) == Imports.E1.A);
        expect(Imports.roundtripEnum(Imports.E1.B) == Imports.E1.B);

        expect(Imports.invertBool(true) == false);
        expect(Imports.invertBool(false) == true);

        {
            Tuple6<Imports.C1, Imports.C2, Imports.C3, Imports.C4, Imports.C5, Imports.C6> result
                = Imports.variantCasts(new Tuple6<>(Imports.C1.a(1),
                                                            Imports.C2.a(2),
                                                            Imports.C3.a(3),
                                                            Imports.C4.a(4L),
                                                            Imports.C5.a(5L),
                                                            Imports.C6.a(6.0F)));

            expect(result.f0.tag == Imports.C1.A && result.f0.getA() == 1);
            expect(result.f1.tag == Imports.C2.A && result.f1.getA() == 2);
            expect(result.f2.tag == Imports.C3.A && result.f2.getA() == 3);
            expect(result.f3.tag == Imports.C4.A && result.f3.getA() == 4L);
            expect(result.f4.tag == Imports.C5.A && result.f4.getA() == 5L);
            expect(result.f5.tag == Imports.C6.A && result.f5.getA() == 6.0F);
        }

        {
            Tuple6<Imports.C1, Imports.C2, Imports.C3, Imports.C4, Imports.C5, Imports.C6> result
                = Imports.variantCasts(new Tuple6<>(Imports.C1.b(1L),
                                                            Imports.C2.b(2.0F),
                                                            Imports.C3.b(3.0D),
                                                            Imports.C4.b(4.0F),
                                                            Imports.C5.b(5.0D),
                                                            Imports.C6.b(6.0D)));

            expect(result.f0.tag == Imports.C1.B && result.f0.getB() == 1L);
            expect(result.f1.tag == Imports.C2.B && result.f1.getB() == 2.0F);
            expect(result.f2.tag == Imports.C3.B && result.f2.getB() == 3.0D);
            expect(result.f3.tag == Imports.C4.B && result.f3.getB() == 4.0F);
            expect(result.f4.tag == Imports.C5.B && result.f4.getB() == 5.0D);
            expect(result.f5.tag == Imports.C6.B && result.f5.getB() == 6.0D);
        }

        {
            Tuple4<Imports.Z1, Imports.Z2, Imports.Z3, Imports.Z4> result
                = Imports.variantZeros(new Tuple4<>(Imports.Z1.a(1),
                                                            Imports.Z2.a(2L),
                                                            Imports.Z3.a(3.0F),
                                                            Imports.Z4.a(4.0D)));

            expect(result.f0.tag == Imports.Z1.A && result.f0.getA() == 1);
            expect(result.f1.tag == Imports.Z2.A && result.f1.getA() == 2L);
            expect(result.f2.tag == Imports.Z3.A && result.f2.getA() == 3.0F);
            expect(result.f3.tag == Imports.Z4.A && result.f3.getA() == 4.0D);
        }

        {
            Tuple4<Imports.Z1, Imports.Z2, Imports.Z3, Imports.Z4> result
                = Imports.variantZeros(new Tuple4<>(Imports.Z1.b(),
                                                            Imports.Z2.b(),
                                                            Imports.Z3.b(),
                                                            Imports.Z4.b()));

            expect(result.f0.tag == Imports.Z1.B);
            expect(result.f1.tag == Imports.Z2.B);
            expect(result.f2.tag == Imports.Z3.B);
            expect(result.f3.tag == Imports.Z4.B);
        }

        Imports.variantTypedefs(null, false, Result.err(Tuple0.INSTANCE));

        {
            Tuple3<Boolean, Result<Tuple0, Tuple0>, Imports.MyErrno> result
                = Imports.variantEnums(true, Result.ok(Tuple0.INSTANCE), Imports.MyErrno.SUCCESS);

            expect(result.f0 == false);
            expect(result.f1.tag == Result.ERR);
            expect(result.f2 == Imports.MyErrno.A);
        }
    }

    private static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }
}
