using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using wit_numbers.wit.imports.test.numbers.Test;

namespace wit_numbers
{

    public class NumbersWorldImpl : INumbersWorld
    {
        public static void TestImports()
        {
            Debug.Assert(TestInterop.RoundtripU8(1) == 1);
            Debug.Assert(TestInterop.RoundtripU8(0) == 0);
            Debug.Assert(TestInterop.RoundtripU8(Byte.MaxValue) == Byte.MaxValue);

            Debug.Assert(TestInterop.RoundtripS8(1) == 1);
            Debug.Assert(TestInterop.RoundtripS8(SByte.MinValue) == SByte.MinValue);
            Debug.Assert(TestInterop.RoundtripS8(SByte.MaxValue) == SByte.MaxValue);

            Debug.Assert(TestInterop.RoundtripU16(1) == 1);
            Debug.Assert(TestInterop.RoundtripU16(0) == 0);
            Debug.Assert(TestInterop.RoundtripU16(UInt16.MaxValue) == UInt16.MaxValue);

            Debug.Assert(TestInterop.RoundtripS16(1) == 1);
            Debug.Assert(TestInterop.RoundtripS16(Int16.MinValue) == Int16.MinValue);
            Debug.Assert(TestInterop.RoundtripS16(Int16.MaxValue) == Int16.MaxValue);

            Debug.Assert(TestInterop.RoundtripU32(1) == 1);
            Debug.Assert(TestInterop.RoundtripU32(0) == 0);
            Debug.Assert(TestInterop.RoundtripU32(UInt32.MaxValue) == UInt32.MaxValue);

            Debug.Assert(TestInterop.RoundtripS32(1) == 1);
            Debug.Assert(TestInterop.RoundtripS32(Int32.MinValue) == Int32.MinValue);
            Debug.Assert(TestInterop.RoundtripS32(Int32.MaxValue) == Int32.MaxValue);

            Debug.Assert(TestInterop.RoundtripU64(1) == 1);
            Debug.Assert(TestInterop.RoundtripU64(0) == 0);
            Debug.Assert(TestInterop.RoundtripU64(UInt64.MaxValue) == UInt64.MaxValue);

            Debug.Assert(TestInterop.RoundtripS64(1) == 1);
            Debug.Assert(TestInterop.RoundtripS64(Int64.MinValue) == Int64.MinValue);
            Debug.Assert(TestInterop.RoundtripS64(Int64.MaxValue) == Int64.MaxValue);

            Debug.Assert(TestInterop.RoundtripFloat32(1.0f) == 1.0f);
            Debug.Assert(TestInterop.RoundtripFloat32(Single.PositiveInfinity) == Single.PositiveInfinity);
            Debug.Assert(TestInterop.RoundtripFloat32(Single.NegativeInfinity) == Single.NegativeInfinity);
            Debug.Assert(float.IsNaN(TestInterop.RoundtripFloat32(Single.NaN)));

            Debug.Assert(TestInterop.RoundtripFloat64(1.0) == 1.0);
            Debug.Assert(TestInterop.RoundtripFloat64(Double.PositiveInfinity) == Double.PositiveInfinity);
            Debug.Assert(TestInterop.RoundtripFloat64(Double.NegativeInfinity) == Double.NegativeInfinity);
            Debug.Assert(double.IsNaN(TestInterop.RoundtripFloat64(Double.NaN)));

            Debug.Assert(TestInterop.RoundtripChar('a') == 'a');
            Debug.Assert(TestInterop.RoundtripChar(' ') == ' ');
            Debug.Assert(Char.ConvertFromUtf32((int)TestInterop.RoundtripChar((uint)Char.ConvertToUtf32("🚩", 0))) ==
                         "🚩"); // This is 2 chars long as it contains a surrogate pair

            TestInterop.SetScalar(2);
            Debug.Assert(TestInterop.GetScalar() == 2);
            TestInterop.SetScalar(4);
            Debug.Assert(TestInterop.GetScalar() == 4);
        }
    }
}

namespace wit_numbers.wit.exports.test.numbers.Test
{
    public class TestImpl : wit_numbers.wit.exports.test.numbers.Test.ITest
    {
        static uint SCALAR = 0;

        public static byte RoundtripU8(byte p0)
        {
            return p0;
        }

        public static sbyte RoundtripS8(sbyte p0)
        {
            return p0;
        }

        public static ushort RoundtripU16(ushort p0)
        {
            return p0;
        }

        public static short RoundtripS16(short p0)
        {
            return p0;
        }

        public static uint RoundtripU32(uint p0)
        {
            return p0;
        }

        public static int RoundtripS32(int p0)
        {
            return p0;
        }

        public static ulong RoundtripU64(ulong p0)
        {
            return p0;
        }

        public static long RoundtripS64(long p0)
        {
            return p0;
        }

        public static float RoundtripFloat32(float p0)
        {
            return p0;
        }

        public static double RoundtripFloat64(double p0)
        {
            return p0;
        }

        public static uint RoundtripChar(uint p0)
        {
            return p0;
        }

        public static void SetScalar(uint p0)
        {
            SCALAR = p0;
        }

        public static uint GetScalar()
        {
            return SCALAR;
        }
    }
}
    