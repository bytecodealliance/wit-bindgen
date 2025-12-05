using RunnerWorld.wit.imports.test.resourceBorrow;
using System.Diagnostics;

namespace RunnerWorld;

public class RunnerWorldImpl : IRunnerWorld
{
    public static void Run()
    {
        uint ret = ToTestInterop.Foo(new IToTest.Thing(42));
        Debug.Assert(ret == 42 + 1 + 2);
    }
}
