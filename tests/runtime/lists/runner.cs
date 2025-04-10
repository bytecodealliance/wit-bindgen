using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.lists;
using System.Text;


public class Program
{
    public static void Main(string[] args)
    {
        ToTestInterop.EmptyListParam(new byte[0]);
        ToTestInterop.EmptyStringParam("");

        {
            byte[] result = ToTestInterop.EmptyListResult();
            Debug.Assert(result.Length == 0);
        }

        {
            string result = ToTestInterop.EmptyStringResult();
            Debug.Assert(result.Length == 0);
        }

        ToTestInterop.ListParam(new byte[] { (byte)1, (byte)2, (byte)3, (byte)4 });
        ToTestInterop.ListParam((new byte[] { (byte)1, (byte)2, (byte)3, (byte)4 }).AsSpan());
        ToTestInterop.ListParam((new byte[] { (byte)1, (byte)2, (byte)3, (byte)4 }).AsMemory());
        ToTestInterop.ListParam2("foo");
        ToTestInterop.ListParam3(new List<String>() {
                "foo",
                "bar",
                "baz"
            });

        ToTestInterop.ListParam4(new List<List<String>>() {
                new List<String>() {
                    "foo",
                    "bar"
               },
                new List<String>() {
                    "baz"
                }
            });

        List<string> randomStrings = new List<string>();
        for (int i = 0; i < 1000; i++)
        {
            randomStrings.Add(Guid.NewGuid().ToString());
        }
        ToTestInterop.ListParamLarge(randomStrings);

        {
            byte[] result = ToTestInterop.ListResult();
            Debug.Assert(result.Length == 5);
            Debug.Assert(result[0] == (byte)1);
            Debug.Assert(result[1] == (byte)2);
            Debug.Assert(result[2] == (byte)3);
            Debug.Assert(result[3] == (byte)4);
            Debug.Assert(result[4] == (byte)5);
        }

        {
            string result = ToTestInterop.ListResult2();
            Console.WriteLine(result);
            Debug.Assert(result == "hello!");
        }

        {
            List<String> result = ToTestInterop.ListResult3();
            Debug.Assert(result.Count() == 2);
            Console.WriteLine(result[0]);
            Console.WriteLine(result[1]);
            Debug.Assert(result[0] == "hello,");
            Debug.Assert(result[1] == "world!");
        }

        string[] strings = { "x", "", "hello", "hello ⚑ world" };
        foreach (string s in strings)
        {
            string result = ToTestInterop.StringRoundtrip(s);
            Debug.Assert(result == s);

            byte[] bytes = Encoding.UTF8.GetBytes(s);
            Debug.Assert(bytes.SequenceEqual(ToTestInterop.ListRoundtrip(bytes)));
        }

        {
            var (u, s) = ToTestInterop.ListMinmax8(
                new byte[] { byte.MinValue, byte.MaxValue },
                new sbyte[] { sbyte.MinValue, sbyte.MaxValue }
            );

            Debug.Assert(u.Length == 2 && u[0] == byte.MinValue && u[1] == byte.MaxValue);
            Debug.Assert(s.Length == 2 && s[0] == sbyte.MinValue && s[1] == sbyte.MaxValue);
        }

        {
            var (u, s) = ToTestInterop.ListMinmax16(
                new ushort[] { ushort.MinValue, ushort.MaxValue },
                new short[] { short.MinValue, short.MaxValue }
            );

            Console.WriteLine(u[0]);
            Console.WriteLine(u[1]);
            Debug.Assert(u.Length == 2, $"u.Length {u.Length}");
            Debug.Assert(u[0] == ushort.MinValue, $"u[0] == {u[0]}");
            Debug.Assert(u[1] == ushort.MaxValue, $"u[1] == {u[1]}");

            Debug.Assert(s.Length == 2);
            Console.WriteLine(s[0]);
            Console.WriteLine(s[1]);
            Debug.Assert(s.Length == 2 && s[0] == short.MinValue && s[1] == short.MaxValue);
        }

        {
            var (u, s) = ToTestInterop.ListMinmax32(
                new uint[] { uint.MinValue, uint.MaxValue },
                new int[] { int.MinValue, int.MaxValue }
            );

            Debug.Assert(u.Length == 2 && u[0] == uint.MinValue && u[1] == uint.MaxValue);
            Debug.Assert(s.Length == 2 && s[0] == int.MinValue && s[1] == int.MaxValue);
        }

        {
            var (u, s) = ToTestInterop.ListMinmax64(
                new ulong[] { ulong.MinValue, ulong.MaxValue },
                new long[] { long.MinValue, long.MaxValue }
            );

            Debug.Assert(u.Length == 2 && u[0] == ulong.MinValue && u[1] == ulong.MaxValue);

            Debug.Assert(s.Length == 2 && s[0] == long.MinValue && s[1] == long.MaxValue);
        }

        {
            var (u, s) = ToTestInterop.ListMinmaxFloat(
                new float[] {
                        float.MinValue,
                        float.MaxValue,
                        float.NegativeInfinity,
                        float.PositiveInfinity
                },
                new double[] {
                        double.MinValue,
                        double.MaxValue,
                        double.NegativeInfinity,
                        double.PositiveInfinity
                });

            Debug.Assert(u.Length == 4
                && u[0] == float.MinValue
                && u[1] == float.MaxValue
                && u[2] == float.NegativeInfinity
                && u[3] == float.PositiveInfinity);

            Debug.Assert(s.Length == 4
                && s[0] == double.MinValue
                && s[1] == double.MaxValue
                && s[2] == double.NegativeInfinity
                && s[3] == double.PositiveInfinity);
        }
    }
}
