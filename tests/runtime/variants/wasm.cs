using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using VariantsWorld.wit.imports.test.variants;

namespace VariantsWorld
{

    public class VariantsWorldImpl : IVariantsWorld
    {
        public static void TestImports()
        {
            Debug.Assert(TestInterop.RoundtripOption(1.0f).Value == 1);
            Debug.Assert(TestInterop.RoundtripOption(null).HasValue == false);
            Debug.Assert(TestInterop.RoundtripOption(2.0f).Value == 2);

            Debug.Assert(TestInterop.RoundtripResult(Result<uint, float>.ok(2)) == 2.0);
            Debug.Assert(TestInterop.RoundtripResult(Result<uint, float>.ok(4)) == 4.0);
            try {
                TestInterop.RoundtripResult(Result<uint, float>.err(5.3f));
                throw new Exception();
            } catch (WitException e) {
                Debug.Assert((byte)e.Value == 5);
            }

            Debug.Assert(TestInterop.RoundtripEnum(ITest.E1.A) == ITest.E1.A);
            Debug.Assert(TestInterop.RoundtripEnum(ITest.E1.B) == ITest.E1.B);

            Debug.Assert(TestInterop.InvertBool(true) == false);
            Debug.Assert(TestInterop.InvertBool(false) == true);

            var (a1, a2, a3, a4, a5, a6) =
            TestInterop.VariantCasts((ITest.C1.a(1), ITest.C2.a(2), ITest.C3.a(3), ITest.C4.a(4), ITest.C5.a(5), ITest.C6.a(6.0f)));
            Debug.Assert(a1.AsA == 1);
            Debug.Assert(a2.AsA == 2);
            Debug.Assert(a3.AsA == 3);
            Debug.Assert(a4.AsA == 4);
            Debug.Assert(a5.AsA == 5);
            Debug.Assert(a6.AsA == 6.0f);

            var (b1, b2, b3, b4, b5, b6) =
TestInterop.VariantCasts((ITest.C1.b(1), ITest.C2.b(2), ITest.C3.b(3), ITest.C4.b(4), ITest.C5.b(5), ITest.C6.b(6.0)));
            Debug.Assert(b1.AsB == 1);
            Debug.Assert(b2.AsB == 2.0f);
            Debug.Assert(b3.AsB == 3.0f);
            Debug.Assert(b4.AsB == 4.0f);
            Debug.Assert(b5.AsB == 5.0f);
            Debug.Assert(b6.AsB == 6.0);

            var (za1, za2, za3, za4) =
TestInterop.VariantZeros((ITest.Z1.a(1), ITest.Z2.a(2), ITest.Z3.a(3.0f), ITest.Z4.a(4.0f)));
            Debug.Assert(za1.AsA == 1);
            Debug.Assert(za2.AsA == 2);
            Debug.Assert(za3.AsA == 3.0f);
            Debug.Assert(za4.AsA == 4.0f);

            var (zb1, zb2, zb3, zb4) =
TestInterop.VariantZeros((ITest.Z1.b(), ITest.Z2.b(), ITest.Z3.b(), ITest.Z4.b()));
            //TODO: Add comparison operator to variants and None
            //Debug.Assert(zb1.AsB == ITest.Z1.b());
            //Debug.Assert(zb2.AsB == ITest.Z2.b());
            //Debug.Assert(zb3.AsB == ITest.Z3.b());
            //Debug.Assert(zb4.AsB == ITest.Z4.b());

            TestInterop.VariantTypedefs(null, false, Result<uint, None>.err(new None()));

            var (a, b, c) = TestInterop.VariantEnums(true, Result<None, None>.ok(new None()), ITest.MyErrno.SUCCESS);
            Debug.Assert(a == false);
            var test = b.AsErr;
            Debug.Assert(c == ITest.MyErrno.A);
        }
    }
}

namespace VariantsWorld.wit.exports.test.variants
{
    public class TestImpl : ITest
    {
        public static byte? RoundtripOption(float? a)
        {
            return a is null ? null : (byte)a;
        }

        public static double RoundtripResult(Result<uint, float> a)
        {
            switch (a.Tag)
            {
                case Result<double, byte>.OK: return (double)a.AsOk;
                case Result<double, byte>.ERR: throw new WitException((byte)a.AsErr, 0);
                default: throw new ArgumentException();
            }
        }

        public static ITest.E1 RoundtripEnum(ITest.E1 a)
        {
            return a;
        }

        public static bool InvertBool(bool a)
        {
            return !a;
        }

        public static (ITest.C1, ITest.C2, ITest.C3, ITest.C4, ITest.C5, ITest.C6)
            VariantCasts((ITest.C1, ITest.C2, ITest.C3, ITest.C4, ITest.C5, ITest.C6) a)
        {
            return a;
        }

        public static (bool, Result<None, None>, ITest.MyErrno)
            VariantEnums(bool a, Result<None, None> b, ITest.MyErrno c)
        {
            return new(a, b, c);
        }

        public static void VariantTypedefs(uint? a, bool b, Result<uint, None> c) { }

        public static (ITest.Z1, ITest.Z2, ITest.Z3, ITest.Z4) VariantZeros((ITest.Z1, ITest.Z2, ITest.Z3, ITest.Z4) a)
        {
            return a;
        }
    }
}
