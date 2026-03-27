namespace TestWorld.wit.Exports.test.records
{
    public class ToTestExportsImpl : IToTestExports
    {
        public static (byte, ushort) MultipleResults()
        {
            return (4, 5);
        }

        public static (uint, byte) SwapTuple((byte, uint) a)
        {
            return (a.Item2, a.Item1);
        }

        public static IToTestExports.F1 RoundtripFlags1(
            IToTestExports.F1 a)
        {
            return a;
        }

        public static IToTestExports.F2 RoundtripFlags2(
            IToTestExports.F2 a)
        {
            return a;
        }

        public static (IToTestExports.Flag8,
            IToTestExports.Flag16,
            IToTestExports.Flag32) RoundtripFlags3(
                IToTestExports.Flag8 a,
                IToTestExports.Flag16 b,
                IToTestExports.Flag32 c)
        {
            return (a, b, c);
        }

        public static IToTestExports.R1 RoundtripRecord1(
            IToTestExports.R1 a)
        {
            return a;
        }

        public static byte Tuple1(byte a)
        {
            return a;
        }
    }
}