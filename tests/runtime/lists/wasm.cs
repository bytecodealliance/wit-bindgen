using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using ListsWorld.wit.imports.test.lists;

namespace ListsWorld {

    public class ListsWorldImpl : IListsWorld
    {
        public static uint AllocatedBytes()
        {
            return 0;
        }

        public static void TestImports()
        {

            TestInterop.EmptyListParam(new byte[0]);
            TestInterop.EmptyStringParam("");
            
            {
                byte[] result = TestInterop.EmptyListResult();
                Debug.Assert(result.Length == 0);
            }

            {
                string result = TestInterop.EmptyStringResult();
                Debug.Assert(result.Length == 0);
            }

            TestInterop.ListParam(new byte[] { (byte)1, (byte)2, (byte)3, (byte)4 });
            TestInterop.ListParam2("foo");
            TestInterop.ListParam3(new List<String>() {
                "foo",
                "bar",
                "baz"
            });

            TestInterop.ListParam4(new List<List<String>>() {
                new List<String>() {
                    "foo",
                    "bar"
               },
                new List<String>() {
                    "baz"
                }
            });

            {
               byte[] result = TestInterop.ListResult();
               Debug.Assert(result.Length == 5);
               Debug.Assert(result[0] == (byte)1);
               Debug.Assert(result[1] == (byte)2);
               Debug.Assert(result[2] == (byte)3);
               Debug.Assert(result[3] == (byte)4);
               Debug.Assert(result[4] == (byte)5);
            }

            {
               string result = TestInterop.ListResult2();
               Console.WriteLine(result);
               Debug.Assert(result == "hello!");
            }
            
            {
               List<String> result = TestInterop.ListResult3();
               Debug.Assert(result.Count() == 2);
               Console.WriteLine(result[0]);
               Console.WriteLine(result[1]);
               Debug.Assert(result[0] == "hello,");
               Debug.Assert(result[1] == "world!");
            }
            
            //
            //for (string s : new String[] { "x", "", "hello", "hello ⚑ world" })
            //{
            //    string result = Test.stringRoundtrip(s);
            //    Debug.Assert(result.equals(s));
            //
            //    byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
            //    Debug.Assert(Arrays.equals(bytes, Test.listRoundtrip(bytes)));
            //}
            //
            //{
            //    Tuple2<byte[], byte[]> result = Test.listMinmax8
            //        (new byte[] { (byte)0, (byte)0xFF }, new byte[] { (byte)0x80, (byte)0x7F });
            //
            //    Debug.Assert(result.f0.length == 2 && result.f0[0] == (byte)0 && result.f0[1] == (byte)0xFF);
            //    Debug.Assert(result.f1.length == 2 && result.f1[0] == (byte)0x80 && result.f1[1] == (byte)0x7F);
            //}
            //
            //{
            //    Tuple2<short[], short[]> result = Test.listMinmax16
            //        (new short[] { (short)0, (short)0xFFFF }, new short[] { (short)0x8000, (short)0x7FFF });
            //
            //    Debug.Assert(result.f0.length == 2 && result.f0[0] == (short)0 && result.f0[1] == (short)0xFFFF);
            //    Debug.Assert(result.f1.length == 2 && result.f1[0] == (short)0x8000 && result.f1[1] == (short)0x7FFF);
            //}
            //
            //{
            //    Tuple2<int[], int[]> result = Test.listMinmax32
            //        (new int[] { 0, 0xFFFFFFFF }, new int[] { 0x80000000, 0x7FFFFFFF });
            //
            //    Debug.Assert(result.f0.length == 2 && result.f0[0] == 0 && result.f0[1] == 0xFFFFFFFF);
            //    Debug.Assert(result.f1.length == 2 && result.f1[0] == 0x80000000 && result.f1[1] == 0x7FFFFFFF);
            //}
            //
            //{
            //    Tuple2<long[], long[]> result = Test.listMinmax64
            //        (new long[] { 0, 0xFFFFFFFFFFFFFFFFL }, new long[] { 0x8000000000000000L, 0x7FFFFFFFFFFFFFFFL });
            //
            //    Debug.Assert(result.f0.length == 2
            //           && result.f0[0] == 0
            //           && result.f0[1] == 0xFFFFFFFFFFFFFFFFL);
            //
            //    Debug.Assert(result.f1.length == 2
            //           && result.f1[0] == 0x8000000000000000L
            //           && result.f1[1] == 0x7FFFFFFFFFFFFFFFL);
            //}
            //
            //{
            //    Tuple2<float[], double[]> result = Test.listMinmaxFloat
            //        (new float[] {
            //            -Float.MAX_VALUE,
            //            Float.MAX_VALUE,
            //            Float.NEGATIVE_INFINITY,
            //            Float.POSITIVE_INFINITY
            //        },
            //            new double[] {
            //                -Double.MAX_VALUE,
            //                Double.MAX_VALUE,
            //                Double.NEGATIVE_INFINITY,
            //                Double.POSITIVE_INFINITY
            //            });
            //
            //    Debug.Assert(result.f0.length == 4
            //           && result.f0[0] == -Float.MAX_VALUE
            //           && result.f0[1] == Float.MAX_VALUE
            //           && result.f0[2] == Float.NEGATIVE_INFINITY
            //           && result.f0[3] == Float.POSITIVE_INFINITY);
            //
            //    Debug.Assert(result.f1.length == 4
            //           && result.f1[0] == -Double.MAX_VALUE
            //           && result.f1[1] == Double.MAX_VALUE
            //           && result.f1[2] == Double.NEGATIVE_INFINITY
            //           && result.f1[3] == Double.POSITIVE_INFINITY);
            //}
        }

    }
}

namespace ListsWorld.wit.exports.test.lists
{
    public class TestImpl : ITest
    {

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
