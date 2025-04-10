using System.Diagnostics;

namespace TestWorld.wit.exports.test.options
{
    public class ToTestImpl : IToTest
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
