using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.numbers;

public class Program
{
    public static void Main(string[] args)
    {
        Debug.Assert(NumbersInterop.RoundtripU8(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripU8(0) == 0);
        Debug.Assert(NumbersInterop.RoundtripU8(Byte.MaxValue) == Byte.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripS8(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripS8(SByte.MinValue) == SByte.MinValue);
        Debug.Assert(NumbersInterop.RoundtripS8(SByte.MaxValue) == SByte.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripU16(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripU16(0) == 0);
        Debug.Assert(NumbersInterop.RoundtripU16(UInt16.MaxValue) == UInt16.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripS16(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripS16(Int16.MinValue) == Int16.MinValue);
        Debug.Assert(NumbersInterop.RoundtripS16(Int16.MaxValue) == Int16.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripU32(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripU32(0) == 0);
        Debug.Assert(NumbersInterop.RoundtripU32(UInt32.MaxValue) == UInt32.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripS32(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripS32(Int32.MinValue) == Int32.MinValue);
        Debug.Assert(NumbersInterop.RoundtripS32(Int32.MaxValue) == Int32.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripU64(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripU64(0) == 0);
        Debug.Assert(NumbersInterop.RoundtripU64(UInt64.MaxValue) == UInt64.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripS64(1) == 1);
        Debug.Assert(NumbersInterop.RoundtripS64(Int64.MinValue) == Int64.MinValue);
        Debug.Assert(NumbersInterop.RoundtripS64(Int64.MaxValue) == Int64.MaxValue);

        Debug.Assert(NumbersInterop.RoundtripF32(1.0f) == 1.0f);
        Debug.Assert(NumbersInterop.RoundtripF32(Single.PositiveInfinity) == Single.PositiveInfinity);
        Debug.Assert(NumbersInterop.RoundtripF32(Single.NegativeInfinity) == Single.NegativeInfinity);
        Debug.Assert(float.IsNaN(NumbersInterop.RoundtripF32(Single.NaN)));

        Debug.Assert(NumbersInterop.RoundtripF64(1.0) == 1.0);
        Debug.Assert(NumbersInterop.RoundtripF64(Double.PositiveInfinity) == Double.PositiveInfinity);
        Debug.Assert(NumbersInterop.RoundtripF64(Double.NegativeInfinity) == Double.NegativeInfinity);
        Debug.Assert(double.IsNaN(NumbersInterop.RoundtripF64(Double.NaN)));

        Debug.Assert(NumbersInterop.RoundtripChar('a') == 'a');
        Debug.Assert(NumbersInterop.RoundtripChar(' ') == ' ');
        Debug.Assert(Char.ConvertFromUtf32((int)NumbersInterop.RoundtripChar((uint)Char.ConvertToUtf32("🚩", 0))) ==
                     "🚩"); // This is 2 chars long as it contains a surrogate pair

        NumbersInterop.SetScalar(2);
        Debug.Assert(NumbersInterop.GetScalar() == 2);
        NumbersInterop.SetScalar(4);
        Debug.Assert(NumbersInterop.GetScalar() == 4);
    }
}
