using System.Diagnostics;
using RecordsWorld.wit.imports.test.records;

namespace RecordsWorld
{
    public class RecordsWorldImpl : IRecordsWorld
    {
        public static void TestImports()
        {
            {
                var results = TestInterop.MultipleResults();
                Debug.Assert(results.Item1 == 4);
                Debug.Assert(results.Item2 == 5);
            }

            (byte, uint) input = (1, 2);
            (uint, byte) output = TestInterop.SwapTuple(input);
            Debug.Assert(output.Item1 == 2);
            Debug.Assert(output.Item2 == 1);

            Debug.Assert(TestInterop.RoundtripFlags1(ITest.F1.A) == ITest.F1.A);
            Debug.Assert(TestInterop.RoundtripFlags1(0) == 0);
            Debug.Assert(TestInterop.RoundtripFlags1(ITest.F1.B) == ITest.F1.B);
            Debug.Assert(TestInterop.RoundtripFlags1(ITest.F1.A | ITest.F1.B) == (ITest.F1.A | ITest.F1.B));

            Debug.Assert(TestInterop.RoundtripFlags2(ITest.F2.C) == ITest.F2.C);
            Debug.Assert(TestInterop.RoundtripFlags2(0) == 0);
            Debug.Assert(TestInterop.RoundtripFlags2(ITest.F2.D) == ITest.F2.D);
            Debug.Assert(TestInterop.RoundtripFlags2(ITest.F2.C | ITest.F2.E) == (ITest.F2.C | ITest.F2.E));

            {
                var result = TestInterop.RoundtripFlags3(ITest.Flag8.B0, ITest.Flag16.B1,
                    ITest.Flag32.B2);
                Debug.Assert(result.Item1 == ITest.Flag8.B0);
                Debug.Assert(result.Item2 == ITest.Flag16.B1);
                Debug.Assert(result.Item3 == ITest.Flag32.B2);
            }

            {
                ITest.R1 inputRecord = new(8, 0);
                var result = TestInterop.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 8);
                Debug.Assert(result.b == 0);
            }

            {
                ITest.R1 inputRecord = new(0, ITest.F1.A | ITest.F1.B);

                var result = TestInterop.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 0);
                Debug.Assert(result.b == (ITest.F1.A | ITest.F1.B));
            }

            {
                var result = TestInterop.Tuple1(1);
                Debug.Assert(result == 1);
            }
        }
    }

}

namespace RecordsWorld.wit.exports.test.records
{
    public class TestImpl : ITest
    {
        public static (byte, ushort) MultipleResults()
        {
            return (100, 200);
        }

        public static (uint, byte) SwapTuple((byte, uint) a)
        {
            return (a.Item2, a.Item1);
        }

        public static ITest.F1 RoundtripFlags1(
            ITest.F1 a)
        {
            return a;
        }

        public static ITest.F2 RoundtripFlags2(
            ITest.F2 a)
        {
            return a;
        }

        public static (ITest.Flag8,
            ITest.Flag16,
            ITest.Flag32) RoundtripFlags3(
                ITest.Flag8 a,
                ITest.Flag16 b,
                ITest.Flag32 c)
        {
            return (a, b, c);
        }

        public static ITest.R1 RoundtripRecord1(
            ITest.R1 a)
        {
            return a;
        }

        public static byte Tuple1(byte a)
        {
            return a;
        }
    }
}
