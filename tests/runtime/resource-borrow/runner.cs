using RunnerWorld.wit.Imports.test.resourceBorrow;
using System.Diagnostics;

namespace RunnerWorld;

public class RunnerWorldImpl : IRunnerWorld
{
    public static void Run()
    {
        uint ret = ToTestImports.Foo(new IToTestImports.Thing(42));
        Debug.Assert(ret == 42 + 1 + 2);
    }
}
