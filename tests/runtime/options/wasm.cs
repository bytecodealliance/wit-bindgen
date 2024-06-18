using System.Diagnostics;
using OptionsWorld.wit.imports.test.options;

namespace OptionsWorld.wit.exports.test.options
{
    public class TestImpl : ITest
    {
        public static void OptionNoneParam(string? a)
        {
            Debug.Assert(a == null);
        }
        
        public static string? OptionNoneResult()
        {
            return null;
        }
        
        public static void OptionSomeParam(string? a)
        {
            Debug.Assert(a == "foo");
        }
        
        public static string? OptionSomeResult()
        {
            return "foo";
        }

        public static string? OptionRoundtrip(string? a)
        {
            return a;
        }        

        public static Option<uint?> DoubleOptionRoundtrip(Option<uint?> a)
        {
            return a;
        }        
    }
}

namespace OptionsWorld
{
    public class OptionsWorldImpl : IOptionsWorld
    {
        public static void TestImports()
        {
            TestInterop.OptionNoneParam(null);
            TestInterop.OptionSomeParam("foo");
            Debug.Assert(TestInterop.OptionNoneResult() == null);
            Debug.Assert(TestInterop.OptionSomeResult() == "foo");
            Debug.Assert(TestInterop.OptionRoundtrip("foo") == "foo");
            Debug.Assert(TestInterop.DoubleOptionRoundtrip(new Option<uint?>(42)).Value == 42);
            Debug.Assert(TestInterop.DoubleOptionRoundtrip(new Option<uint?>(null)).Value == null);
            Debug.Assert(!TestInterop.DoubleOptionRoundtrip(Option<uint?>.None).HasValue);
        }
    }
}
