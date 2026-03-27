using System.Diagnostics;
using v1 = RunnerWorld.wit.Imports.test.dep.v0_1_0;
using v2 = RunnerWorld.wit.Imports.test.dep.v0_2_0;
using System.Text;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        Debug.Assert(v1.ITestImports.X() == 1.0f);
        Debug.Assert(v1.ITestImports.Y(1.0f) == 2.0f);

        Debug.Assert(v2.ITestImports.X() == 2.0f);
        Debug.Assert(v2.ITestImports.Z(1.0f, 1.0f) == 4.0f);
    }
}
