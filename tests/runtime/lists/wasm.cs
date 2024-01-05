using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using wit_lists.wit.imports.test.lists.Test;

namespace wit_lists;

public class ListsWorldImpl : IListsWorld
{
    public static int AllocatedBytes()
    {
        return 0;
    }

    public static void TestImports()
    {
        //Test.EmptyListParam(new byte[0]);
        //
        //Test.EmptyStringParam("");
        //
        //{
        //    byte[] result = Test.EmptyListResult();
        //    Debug.Assert(result.length == 0);
        //}
        //
        //{
        //    string result = Test.EmptyStringResult();
        //    Debug.Assert(result.length() == 0);
        //}
        //
        //Test.ListParam(new byte[] { (byte)1, (byte)2, (byte)3, (byte)4 });
        //
        //Test.ListParam2("foo");
        //
        //        Test.listParam3(new List<String>() {{
        //            add("foo");
        //        add("bar");
        //        add("baz");
        //    }
        //});

        //        Test.listParam4(new List<List<String>>() {{
        //            add(new List<String>() {{
        //                add("foo");
        //        add("bar");
        //    }
        //});
        //add(new List<String>() {{
        //                add("baz");
        //            }});
        //        }});

        //{
        //    byte[] result = Test.listResult();
        //    Debug.Assert(result.length == 5);
        //    Debug.Assert(result[0] == (byte)1);
        //    Debug.Assert(result[1] == (byte)2);
        //    Debug.Assert(result[2] == (byte)3);
        //    Debug.Assert(result[3] == (byte)4);
        //    Debug.Assert(result[4] == (byte)5);
        //}
        //
        //{
        //    string result = Test.listResult2();
        //    Debug.Assert(result.equals("hello!"));
        //}
        //
        //{
        //    List<String> result = Test.listResult3();
        //    Debug.Assert(result.size() == 2);
        //    Debug.Assert(result.get(0).equals("hello,"));
        //    Debug.Assert(result.get(1).equals("world!"));
        //}
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

    public static void Expect(bool v)
    {
        if (!v)
        {
            //throw new AssertionError();
        }
    }
}

public class ListsImpl : wit_lists.wit.exports.test.lists.Test.ITest
{

    public static void EmptyListParam(byte[] a)
    {
        Debug.Assert(a.length == 0);
    }

    public static void EmptyStringParam(string a)
    {
        Debug.Assert(a.length() == 0);
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
        Debug.Assert(a.length == 4);
        Debug.Assert(a[0] == 1);
        Debug.Assert(a[1] == 2);
        Debug.Assert(a[2] == 3);
        Debug.Assert(a[3] == 4);
    }

    public static void ListParam2(string a)
    {
        Debug.Assert(a.equals("foo"));
    }

    public static void ListParam3(List<String> a)
    {
        Debug.Assert(a.size() == 3);
        Debug.Assert(a.get(0).equals("foo"));
        Debug.Assert(a.get(1).equals("bar"));
        Debug.Assert(a.get(2).equals("baz"));
    }

    public static void ListParam4(List<List<String>> a)
    {
        Debug.Assert(a.size() == 2);
        Debug.Assert(a.get(0).size() == 2);
        Debug.Assert(a.get(1).size() == 1);

        Debug.Assert(a.get(0).get(0).equals("foo"));
        Debug.Assert(a.get(0).get(1).equals("bar"));
        Debug.Assert(a.get(1).get(0).equals("baz"));
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
}
