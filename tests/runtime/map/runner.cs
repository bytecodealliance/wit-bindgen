//@ wasmtime-flags = '-Wcomponent-model-map'

using System.Diagnostics;
using RunnerWorld.wit.Imports.test.maps;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        TestNamedRoundtrip();
        TestBytesRoundtrip();
        TestEmptyRoundtrip();
        TestOptionRoundtrip();
        TestRecordRoundtrip();
        TestInlineRoundtrip();
        TestLargeRoundtrip();
        TestMultiParamRoundtrip();
        TestNestedRoundtrip();
        TestVariantRoundtrip();
        TestResultRoundtrip();
        TestTupleRoundtrip();
        TestSingleEntryRoundtrip();
    }

    static void TestNamedRoundtrip()
    {
        var input = new Dictionary<uint, string>
        {
            [1] = "uno",
            [2] = "two",
        };
        var ids = IToTestImports.NamedRoundtrip(input);
        Debug.Assert(ids.Count == 2);
        Debug.Assert(ids["uno"] == 1);
        Debug.Assert(ids["two"] == 2);
    }

    static void TestBytesRoundtrip()
    {
        var input = new Dictionary<string, byte[]>
        {
            ["hello"] = System.Text.Encoding.UTF8.GetBytes("world"),
            ["bin"] = new byte[] { 0, 1, 2 },
        };
        var result = IToTestImports.BytesRoundtrip(input);
        Debug.Assert(result.Count == 2);
        Debug.Assert(result["hello"].SequenceEqual(System.Text.Encoding.UTF8.GetBytes("world")));
        Debug.Assert(result["bin"].SequenceEqual(new byte[] { 0, 1, 2 }));
    }

    static void TestEmptyRoundtrip()
    {
        var empty = new Dictionary<uint, string>();
        var result = IToTestImports.EmptyRoundtrip(empty);
        Debug.Assert(result.Count == 0);
    }

    static void TestOptionRoundtrip()
    {
        var input = new Dictionary<string, uint?>
        {
            ["some"] = 42,
            ["none"] = null,
        };
        var result = IToTestImports.OptionRoundtrip(input);
        Debug.Assert(result.Count == 2);
        Debug.Assert(result["some"] == 42);
        Debug.Assert(result["none"] == null);
    }

    static void TestRecordRoundtrip()
    {
        var values = new Dictionary<uint, string>
        {
            [10] = "ten",
            [20] = "twenty",
        };
        var entry = new IToTestImports.LabeledEntry("test-label", values);
        var result = IToTestImports.RecordRoundtrip(entry);
        Debug.Assert(result.label == "test-label");
        Debug.Assert(result.values.Count == 2);
        Debug.Assert(result.values[10] == "ten");
        Debug.Assert(result.values[20] == "twenty");
    }

    static void TestInlineRoundtrip()
    {
        var input = new Dictionary<uint, string>
        {
            [1] = "one",
            [2] = "two",
        };
        var result = IToTestImports.InlineRoundtrip(input);
        Debug.Assert(result.Count == 2);
        Debug.Assert(result["one"] == 1);
        Debug.Assert(result["two"] == 2);
    }

    static void TestLargeRoundtrip()
    {
        var input = new Dictionary<uint, string>();
        for (uint i = 0; i < 100; i++)
        {
            input[i] = $"value-{i}";
        }
        var result = IToTestImports.LargeRoundtrip(input);
        Debug.Assert(result.Count == 100);
        for (uint i = 0; i < 100; i++)
        {
            Debug.Assert(result[i] == $"value-{i}");
        }
    }

    static void TestMultiParamRoundtrip()
    {
        var names = new Dictionary<uint, string>
        {
            [1] = "one",
            [2] = "two",
        };
        var bytes = new Dictionary<string, byte[]>
        {
            ["key"] = new byte[] { 42 },
        };
        var (ids, bytesOut) = IToTestImports.MultiParamRoundtrip(names, bytes);
        Debug.Assert(ids.Count == 2);
        Debug.Assert(ids["one"] == 1);
        Debug.Assert(ids["two"] == 2);
        Debug.Assert(bytesOut.Count == 1);
        Debug.Assert(bytesOut["key"].SequenceEqual(new byte[] { 42 }));
    }

    static void TestNestedRoundtrip()
    {
        var innerA = new Dictionary<uint, string>
        {
            [1] = "one",
            [2] = "two",
        };
        var innerB = new Dictionary<uint, string>
        {
            [10] = "ten",
        };
        var outer = new Dictionary<string, Dictionary<uint, string>>
        {
            ["group-a"] = innerA,
            ["group-b"] = innerB,
        };
        var result = IToTestImports.NestedRoundtrip(outer);
        Debug.Assert(result.Count == 2);
        Debug.Assert(result["group-a"][1] == "one");
        Debug.Assert(result["group-a"][2] == "two");
        Debug.Assert(result["group-b"][10] == "ten");
    }

    static void TestVariantRoundtrip()
    {
        var map = new Dictionary<uint, string> { [1] = "one" };
        var asMap = IToTestImports.VariantRoundtrip(IToTestImports.MapOrString.AsMap(map));
        Debug.Assert(asMap.Tag == IToTestImports.MapOrString.Tags.AsMap);
        Debug.Assert(asMap.AsAsMap[1] == "one");

        var asStr = IToTestImports.VariantRoundtrip(IToTestImports.MapOrString.AsString("hello"));
        Debug.Assert(asStr.Tag == IToTestImports.MapOrString.Tags.AsString);
        Debug.Assert(asStr.AsAsString == "hello");
    }

    static void TestResultRoundtrip()
    {
        var map = new Dictionary<uint, string> { [5] = "five" };
        var ok = IToTestImports.ResultRoundtrip(Result<Dictionary<uint, string>, string>.Ok(map));
        Debug.Assert(ok[5] == "five");

        try
        {
            IToTestImports.ResultRoundtrip(Result<Dictionary<uint, string>, string>.Err("bad input"));
            Debug.Assert(false, "expected exception");
        }
        catch (WitException<string> e)
        {
            Debug.Assert(e.TypedValue == "bad input");
        }
    }

    static void TestTupleRoundtrip()
    {
        var map = new Dictionary<uint, string> { [7] = "seven" };
        var (resultMap, resultNum) = IToTestImports.TupleRoundtrip((map, 42UL));
        Debug.Assert(resultMap.Count == 1);
        Debug.Assert(resultMap[7] == "seven");
        Debug.Assert(resultNum == 42UL);
    }

    static void TestSingleEntryRoundtrip()
    {
        var input = new Dictionary<uint, string> { [99] = "ninety-nine" };
        var result = IToTestImports.SingleEntryRoundtrip(input);
        Debug.Assert(result.Count == 1);
        Debug.Assert(result[99] == "ninety-nine");
    }
}
