using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.records;

namespace RunnerWorld;

public class RunnerWorldImpl : IRunnerWorld
{
    public static void Run()
    {
            {
                var results = IToTestImports.MultipleResults();
                Debug.Assert(results.Item1 == 4);
                Debug.Assert(results.Item2 == 5);
            }

            (byte, uint) input = (1, 2);
            (uint, byte) output = IToTestImports.SwapTuple(input);
            Debug.Assert(output.Item1 == 2);
            Debug.Assert(output.Item2 == 1);

            Debug.Assert(IToTestImports.RoundtripFlags1(IToTestImports.F1.A) == IToTestImports.F1.A);
            Debug.Assert(IToTestImports.RoundtripFlags1(0) == 0);
            Debug.Assert(IToTestImports.RoundtripFlags1(IToTestImports.F1.B) == IToTestImports.F1.B);
            Debug.Assert(IToTestImports.RoundtripFlags1(IToTestImports.F1.A | IToTestImports.F1.B) == (IToTestImports.F1.A | IToTestImports.F1.B));

            Debug.Assert(IToTestImports.RoundtripFlags2(IToTestImports.F2.C) == IToTestImports.F2.C);
            Debug.Assert(IToTestImports.RoundtripFlags2(0) == 0);
            Debug.Assert(IToTestImports.RoundtripFlags2(IToTestImports.F2.D) == IToTestImports.F2.D);
            Debug.Assert(IToTestImports.RoundtripFlags2(IToTestImports.F2.C | IToTestImports.F2.E) == (IToTestImports.F2.C | IToTestImports.F2.E));

            {
                var result = IToTestImports.RoundtripFlags3(IToTestImports.Flag8.B0, IToTestImports.Flag16.B1,
                    IToTestImports.Flag32.B2);
                Debug.Assert(result.Item1 == IToTestImports.Flag8.B0);
                Debug.Assert(result.Item2 == IToTestImports.Flag16.B1);
                Debug.Assert(result.Item3 == IToTestImports.Flag32.B2);
            }

            {
                IToTestImports.R1 inputRecord = new(8, 0);
                var result = IToTestImports.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 8);
                Debug.Assert(result.b == 0);
            }

            {
                IToTestImports.R1 inputRecord = new(0, IToTestImports.F1.A | IToTestImports.F1.B);

                var result = IToTestImports.RoundtripRecord1(inputRecord);
                Debug.Assert(result.a == 0);
                Debug.Assert(result.b == (IToTestImports.F1.A | IToTestImports.F1.B));
            }

            {
                var result = IToTestImports.Tuple1(1);
                Debug.Assert(result == 1);
            }
    }
}
