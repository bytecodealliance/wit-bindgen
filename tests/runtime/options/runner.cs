using System.Diagnostics;
using RunnerWorld.wit.imports.test.options;

namespace RunnerWorld
{
    public class Program
    {
        public static void Main()
        {
            ToTestInterop.OptionNoneParam(null);
            ToTestInterop.OptionSomeParam("foo");
            Debug.Assert(ToTestInterop.OptionNoneResult() == null);
            Debug.Assert(ToTestInterop.OptionSomeResult() == "foo");
            Debug.Assert(ToTestInterop.OptionRoundtrip("foo") == "foo");
            Debug.Assert(ToTestInterop.DoubleOptionRoundtrip(new Option<uint?>(42)).Value == 42);
            Debug.Assert(ToTestInterop.DoubleOptionRoundtrip(new Option<uint?>(null)).Value == null);
            Debug.Assert(!ToTestInterop.DoubleOptionRoundtrip(Option<uint?>.None).HasValue);
        }
    }
}
