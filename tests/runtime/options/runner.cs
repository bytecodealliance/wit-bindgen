using System.Diagnostics;
using RunnerWorld.wit.Imports.test.options;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
            IToTestImports.OptionNoneParam(null);
            IToTestImports.OptionSomeParam("foo");
            Debug.Assert(IToTestImports.OptionNoneResult() == null);
            Debug.Assert(IToTestImports.OptionSomeResult() == "foo");
            Debug.Assert(IToTestImports.OptionRoundtrip("foo") == "foo");
            Debug.Assert(IToTestImports.DoubleOptionRoundtrip(new Option<uint?>(42)).Value == 42);
            Debug.Assert(IToTestImports.DoubleOptionRoundtrip(new Option<uint?>(null)).Value == null);
            Debug.Assert(!IToTestImports.DoubleOptionRoundtrip(Option<uint?>.None).HasValue);
    }
}
