using System;
using System.Runtime.InteropServices;
using System.Diagnostics;

namespace TestWorld.wit.exports.test.numbers
{
    public class NumbersImpl : ITestWorld
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

        public static float RoundtripF32(float p0)
        {
            return p0;
        }

        public static double RoundtripF64(double p0)
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

