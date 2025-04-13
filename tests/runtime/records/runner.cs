using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.records;
using RunnerWorld;

public class Program {
    public static void Main()
    {
        {
                var results = ToTestInterop.MultipleResults();
                Debug.Assert(results.Item1 == 4);
                Debug.Assert(results.Item2 == 5);
            }

            (byte, uint) input = (1, 2);
            (uint, byte) output = ToTestInterop.SwapTuple(input);
            Debug.Assert(output.Item1 == 2);
            Debug.Assert(output.Item2 == 1);

            Debug.Assert(ToTestInterop.RoundtripFlags1(IToTest.F1.A) == IToTest.F1.A);
            Debug.Assert(ToTestInterop.RoundtripFlags1(0) == 0);
            Debug.Assert(ToTestInterop.RoundtripFlags1(IToTest.F1.B) == IToTest.F1.B);
            Debug.Assert(ToTestInterop.RoundtripFlags1(IToTest.F1.A | IToTest.F1.B) == (IToTest.F1.A | IToTest.F1.B));

            Debug.Assert(ToTestInterop.RoundtripFlags2(IToTest.F2.C) == IToTest.F2.C);
            Debug.Assert(ToTestInterop.RoundtripFlags2(0) == 0);
            Debug.Assert(ToTestInterop.RoundtripFlags2(IToTest.F2.D) == IToTest.F2.D);
            Debug.Assert(ToTestInterop.RoundtripFlags2(IToTest.F2.C | IToTest.F2.E) == (IToTest.F2.C | IToTest.F2.E));

            {
                var result = ToTestInterop.RoundtripFlags3(IToTest.Flag8.B0, IToTest.Flag16.B1,
                    IToTest.Flag32.B2);
                Debug.Assert(result.Item1 == IToTest.Flag8.B0);
                Debug.Assert(result.Item2 == IToTest.Flag16.B1);
                Debug.Assert(result.Item3 == IToTest.Flag32.B2);
            }

            {
                IToTest.R1 inputRecord = new(8, 0);
                var result = ToTestInterop.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 8);
                Debug.Assert(result.b == 0);
            }

            {
                IToTest.R1 inputRecord = new(0, IToTest.F1.A | IToTest.F1.B);

                var result = ToTestInterop.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 0);
                Debug.Assert(result.b == (IToTest.F1.A | IToTest.F1.B));
            }

            {
                var result = ToTestInterop.Tuple1(1);
                Debug.Assert(result == 1);
            }
    }
}