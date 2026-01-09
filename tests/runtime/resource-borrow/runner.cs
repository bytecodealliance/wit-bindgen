using RunnerWorld.wit.Imports.test.resourceBorrow;
using System.Diagnostics;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        uint ret = IToTestImports.Foo(new IToTestImports.Thing(42));
        Debug.Assert(ret == 42 + 1 + 2);
    }
}
