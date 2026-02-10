using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.numbers;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        Debug.Assert(INumbersImports.RoundtripU8(1) == 1);
        Debug.Assert(INumbersImports.RoundtripU8(0) == 0);
        Debug.Assert(INumbersImports.RoundtripU8(Byte.MaxValue) == Byte.MaxValue);

        Debug.Assert(INumbersImports.RoundtripS8(1) == 1);
        Debug.Assert(INumbersImports.RoundtripS8(SByte.MinValue) == SByte.MinValue);
        Debug.Assert(INumbersImports.RoundtripS8(SByte.MaxValue) == SByte.MaxValue);

        Debug.Assert(INumbersImports.RoundtripU16(1) == 1);
        Debug.Assert(INumbersImports.RoundtripU16(0) == 0);
        Debug.Assert(INumbersImports.RoundtripU16(UInt16.MaxValue) == UInt16.MaxValue);

        Debug.Assert(INumbersImports.RoundtripS16(1) == 1);
        Debug.Assert(INumbersImports.RoundtripS16(Int16.MinValue) == Int16.MinValue);
        Debug.Assert(INumbersImports.RoundtripS16(Int16.MaxValue) == Int16.MaxValue);
        Debug.Assert(INumbersImports.RoundtripU32(1) == 1);
        Debug.Assert(INumbersImports.RoundtripU32(0) == 0);
        Debug.Assert(INumbersImports.RoundtripU32(UInt32.MaxValue) == UInt32.MaxValue);

        Debug.Assert(INumbersImports.RoundtripS32(1) == 1);
        Debug.Assert(INumbersImports.RoundtripS32(Int32.MinValue) == Int32.MinValue);
        Debug.Assert(INumbersImports.RoundtripS32(Int32.MaxValue) == Int32.MaxValue);

        Debug.Assert(INumbersImports.RoundtripU64(1) == 1);
        Debug.Assert(INumbersImports.RoundtripU64(0) == 0);
        Debug.Assert(INumbersImports.RoundtripU64(UInt64.MaxValue) == UInt64.MaxValue);

        Debug.Assert(INumbersImports.RoundtripS64(1) == 1);
        Debug.Assert(INumbersImports.RoundtripS64(Int64.MinValue) == Int64.MinValue);
        Debug.Assert(INumbersImports.RoundtripS64(Int64.MaxValue) == Int64.MaxValue);
        Debug.Assert(INumbersImports.RoundtripF32(1.0f) == 1.0f);
        Debug.Assert(INumbersImports.RoundtripF32(Single.PositiveInfinity) == Single.PositiveInfinity);
        Debug.Assert(INumbersImports.RoundtripF32(Single.NegativeInfinity) == Single.NegativeInfinity);
        Debug.Assert(float.IsNaN(INumbersImports.RoundtripF32(Single.NaN)));
        Debug.Assert(INumbersImports.RoundtripF64(1.0) == 1.0);
        Debug.Assert(INumbersImports.RoundtripF64(Double.PositiveInfinity) == Double.PositiveInfinity);
        Debug.Assert(INumbersImports.RoundtripF64(Double.NegativeInfinity) == Double.NegativeInfinity);
        Debug.Assert(double.IsNaN(INumbersImports.RoundtripF64(Double.NaN)));
        Debug.Assert(INumbersImports.RoundtripChar('a') == 'a');
        Debug.Assert(INumbersImports.RoundtripChar(' ') == ' ');
        Debug.Assert(Char.ConvertFromUtf32((int)INumbersImports.RoundtripChar((uint)Char.ConvertToUtf32("🚩", 0))) ==
                     "🚩"); // This is 2 chars long as it contains a surrogate pair

        INumbersImports.SetScalar(2);
        Debug.Assert(INumbersImports.GetScalar() == 2);
        INumbersImports.SetScalar(4);
        Debug.Assert(INumbersImports.GetScalar() == 4);
    }
}
