using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using System.Text;

namespace TestWorld.wit.exports.test.lists
{
    public class ToTestImpl : ITestWorld
    {

        public static uint AllocatedBytes()
        {
            return 0;
        }

        public static void EmptyListParam(byte[] a)
        {
            Debug.Assert(a.Length == 0);
        }

        public static void EmptyStringParam(string a)
        {
            Debug.Assert(a.Length == 0);
        }

        public static byte[] EmptyListResult()
        {
            return new byte[0];
        }

        public static string EmptyStringResult()
        {
            return "";
        }

        public static void ListParam(byte[] a)
        {
            Debug.Assert(a.Length == 4);
            Debug.Assert(a[0] == 1);
            Debug.Assert(a[1] == 2);
            Debug.Assert(a[2] == 3);
            Debug.Assert(a[3] == 4);
        }

        public static void ListParam2(string a)
        {
            Debug.Assert(a.Equals("foo"));
        }

        public static void ListParam3(List<String> a)
        {
            Debug.Assert(a.Count() == 3);
            Debug.Assert(a[0].Equals("foo"));
            Debug.Assert(a[1].Equals("bar"));
            Debug.Assert(a[2].Equals("baz"));
        }

        public static void ListParam4(List<List<String>> a)
        {
            Debug.Assert(a.Count() == 2);
            Debug.Assert(a[0].Count() == 2);
            Debug.Assert(a[1].Count() == 1);

            Debug.Assert(a[0][0].Equals("foo"));
            Debug.Assert(a[0][1].Equals("bar"));
            Debug.Assert(a[1][0].Equals("baz"));
        }

        public static void ListParam5(List<(byte, uint, byte)> a)
        {
            Debug.Assert(a.Count() == 2);
            Debug.Assert(a[0].Item1 == 1);
            Debug.Assert(a[0].Item2 == 2);
            Debug.Assert(a[0].Item3 == 3);
            Debug.Assert(a[1].Item1 == 4);
            Debug.Assert(a[1].Item2 == 5);
            Debug.Assert(a[1].Item3 == 6);
        }

        public static void ListParamLarge(List<String> a)
        {
            Debug.Assert(a.Count() == 1000);
        }

        public static byte[] ListResult()
        {
            return new byte[] { (byte)1, (byte)2, (byte)3, (byte)4, (byte)5 };
        }

        public static string ListResult2()
        {
            return "hello!";
        }

        public static List<string> ListResult3()
        {
            return new List<string>() {
                "hello,",
                "world!"
                };
        }

        public static byte[] ListRoundtrip(byte[] a)
        {
            return a;
        }

        public static string stringRoundtrip(string a)
        {
            return a;
        }

        public static (byte[], byte[]) ListMinmax8(byte[] a, byte[] b)
        {
            return new(a, b);
        }

        public static (short[], short[]) ListMinmax16(short[] a, short[] b)
        {
            return new(a, b);
        }

        public static (int[], int[]) ListMinmax32(int[] a, int[] b)
        {
            return new(a, b);
        }

        public static (long[], long[]) ListMinmax64(long[] a, long[] b)
        {
            return new(a, b);
        }

        public static (float[], double[]) ListMinmaxFloat(float[] a, double[] b)
        {
            return new(a, b);
        }
        public static (byte[], sbyte[]) ListMinmax8(byte[] a, sbyte[] b)
        {
            return (a, b);
        }

        public static (ushort[], short[]) ListMinmax16(ushort[] a, short[] b)
        {
            return (a, b);
        }

        public static (uint[], int[]) ListMinmax32(uint[] a, int[] b)
        {
            return (a, b);
        }

        public static (ulong[], long[]) ListMinmax64(ulong[] a, long[] b)
        {
            return (a, b);
        }

        public static string StringRoundtrip(string a)
        {
            return a;
        }
    }
}
