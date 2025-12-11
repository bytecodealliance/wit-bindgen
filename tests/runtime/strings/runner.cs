using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.strings;
using System.Text;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        IToTestImports.TakeBasic("latin utf16");
        Debug.Assert(IToTestImports.ReturnUnicode() == "🚀🚀🚀 𠈄𓀀");

        Debug.Assert(IToTestImports.ReturnEmpty() == string.Empty);
        Debug.Assert(IToTestImports.Roundtrip("🚀🚀🚀 𠈄𓀀") == "🚀🚀🚀 𠈄𓀀");
    }
}
