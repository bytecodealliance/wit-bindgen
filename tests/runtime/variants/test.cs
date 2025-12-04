namespace TestWorld.wit.Exports.test.variants
{
    public class ToTestExportsImpl : ITestWorldImports
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

        public static IToTestExports.E1 RoundtripEnum(IToTestExports.E1 a)
        {
            return a;
        }

        public static bool InvertBool(bool a)
        {
            return !a;
        }

        public static (IToTestExports.C1, IToTestExports.C2, IToTestExports.C3, IToTestExports.C4, IToTestExports.C5, IToTestExports.C6)
            VariantCasts((IToTestExports.C1, IToTestExports.C2, IToTestExports.C3, IToTestExports.C4, IToTestExports.C5, IToTestExports.C6) a)
        {
            return a;
        }

        public static (bool, Result<None, None>, IToTestExports.MyErrno)
            VariantEnums(bool a, Result<None, None> b, IToTestExports.MyErrno c)
        {
            return new(a, b, c);
        }

        public static void VariantTypedefs(uint? a, bool b, Result<uint, None> c) { }

        public static (IToTestExports.Z1, IToTestExports.Z2, IToTestExports.Z3, IToTestExports.Z4) VariantZeros((IToTestExports.Z1, IToTestExports.Z2, IToTestExports.Z3, IToTestExports.Z4) a)
        {
            return a;
        }
    }
}
