namespace TestWorld.wit.exports.test.records
{
    public class ToTestImpl : ITestWorld
    {
        public static (byte, ushort) MultipleResults()
        {
            return (4, 5);
        }

        public static (uint, byte) SwapTuple((byte, uint) a)
        {
            return (a.Item2, a.Item1);
        }

        public static IToTest.F1 RoundtripFlags1(
            IToTest.F1 a)
        {
            return a;
        }

        public static IToTest.F2 RoundtripFlags2(
            IToTest.F2 a)
        {
            return a;
        }

        public static (IToTest.Flag8,
            IToTest.Flag16,
            IToTest.Flag32) RoundtripFlags3(
                IToTest.Flag8 a,
                IToTest.Flag16 b,
                IToTest.Flag32 c)
        {
            return (a, b, c);
        }

        public static IToTest.R1 RoundtripRecord1(
            IToTest.R1 a)
        {
            return a;
        }

        public static byte Tuple1(byte a)
        {
            return a;
        }
    }
}