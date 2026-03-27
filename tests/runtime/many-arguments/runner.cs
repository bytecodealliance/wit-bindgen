using System;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.manyArguments;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        IToTestImports.ManyArguments(
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16
        );
    }
}

