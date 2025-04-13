namespace TestWorld.wit.exports.test.variants
{
    public class ToTestImpl : ITestWorld
    {
        public static byte? RoundtripOption(float? a)
        {
            return a is null ? null : (byte)a;
        }

        public static double RoundtripResult(Result<uint, float> a)
        {
            switch (a.Tag)
            {
                case Result<double, byte>.Tags.Ok: return (double)a.AsOk;
                case Result<double, byte>.Tags.Err: throw new WitException((byte)a.AsErr, 0);
                default: throw new ArgumentException();
            }
        }

        public static IToTest.E1 RoundtripEnum(IToTest.E1 a)
        {
            return a;
        }

        public static bool InvertBool(bool a)
        {
            return !a;
        }

        public static (IToTest.C1, IToTest.C2, IToTest.C3, IToTest.C4, IToTest.C5, IToTest.C6)
            VariantCasts((IToTest.C1, IToTest.C2, IToTest.C3, IToTest.C4, IToTest.C5, IToTest.C6) a)
        {
            return a;
        }

        public static (bool, Result<None, None>, IToTest.MyErrno)
            VariantEnums(bool a, Result<None, None> b, IToTest.MyErrno c)
        {
            return new(a, b, c);
        }

        public static void VariantTypedefs(uint? a, bool b, Result<uint, None> c) { }

        public static (IToTest.Z1, IToTest.Z2, IToTest.Z3, IToTest.Z4) VariantZeros((IToTest.Z1, IToTest.Z2, IToTest.Z3, IToTest.Z4) a)
        {
            return a;
        }
    }
}
