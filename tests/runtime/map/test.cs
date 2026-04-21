//@ wasmtime-flags = '-Wcomponent-model-map'

using System.Diagnostics;
using TestWorld;

namespace TestWorld.wit.Exports.test.maps
{
    public class ToTestExportsImpl : IToTestExports
    {
        public static Dictionary<string, uint> NamedRoundtrip(Dictionary<uint, string> a)
        {
            Debug.Assert(a[1] == "uno");
            Debug.Assert(a[2] == "two");
            var result = new Dictionary<string, uint>();
            foreach (var entry in a)
            {
                result[entry.Value] = entry.Key;
            }
            return result;
        }

        public static Dictionary<string, byte[]> BytesRoundtrip(Dictionary<string, byte[]> a)
        {
            Debug.Assert(a["hello"].SequenceEqual(System.Text.Encoding.UTF8.GetBytes("world")));
            Debug.Assert(a["bin"].SequenceEqual(new byte[] { 0, 1, 2 }));
            return a;
        }

        public static Dictionary<uint, string> EmptyRoundtrip(Dictionary<uint, string> a)
        {
            Debug.Assert(a.Count == 0);
            return a;
        }

        public static Dictionary<string, uint?> OptionRoundtrip(Dictionary<string, uint?> a)
        {
            Debug.Assert(a["some"] == 42);
            Debug.Assert(a["none"] == null);
            return a;
        }

        public static IToTestExports.LabeledEntry RecordRoundtrip(IToTestExports.LabeledEntry a)
        {
            Debug.Assert(a.label == "test-label");
            Debug.Assert(a.values.Count == 2);
            Debug.Assert(a.values[10] == "ten");
            Debug.Assert(a.values[20] == "twenty");
            return a;
        }

        public static Dictionary<string, uint> InlineRoundtrip(Dictionary<uint, string> a)
        {
            var result = new Dictionary<string, uint>();
            foreach (var entry in a)
            {
                result[entry.Value] = entry.Key;
            }
            return result;
        }

        public static Dictionary<uint, string> LargeRoundtrip(Dictionary<uint, string> a)
        {
            return a;
        }

        public static (Dictionary<string, uint>, Dictionary<string, byte[]>) MultiParamRoundtrip(
            Dictionary<uint, string> a,
            Dictionary<string, byte[]> b)
        {
            Debug.Assert(a.Count == 2);
            Debug.Assert(b.Count == 1);
            var ids = new Dictionary<string, uint>();
            foreach (var entry in a)
            {
                ids[entry.Value] = entry.Key;
            }
            return (ids, b);
        }

        public static Dictionary<string, Dictionary<uint, string>> NestedRoundtrip(
            Dictionary<string, Dictionary<uint, string>> a)
        {
            Debug.Assert(a.Count == 2);
            Debug.Assert(a["group-a"][1] == "one");
            Debug.Assert(a["group-a"][2] == "two");
            Debug.Assert(a["group-b"][10] == "ten");
            return a;
        }

        public static IToTestExports.MapOrString VariantRoundtrip(IToTestExports.MapOrString a)
        {
            return a;
        }

        public static Dictionary<uint, string> ResultRoundtrip(Result<Dictionary<uint, string>, string> a)
        {
            if (a.IsOk)
            {
                return a.AsOk;
            }
            throw new WitException<string>(a.AsErr, 0);
        }

        public static (Dictionary<uint, string>, ulong) TupleRoundtrip((Dictionary<uint, string>, ulong) a)
        {
            Debug.Assert(a.Item1.Count == 1);
            Debug.Assert(a.Item1[7] == "seven");
            Debug.Assert(a.Item2 == 42UL);
            return a;
        }

        public static Dictionary<uint, string> SingleEntryRoundtrip(Dictionary<uint, string> a)
        {
            Debug.Assert(a.Count == 1);
            return a;
        }
    }
}
