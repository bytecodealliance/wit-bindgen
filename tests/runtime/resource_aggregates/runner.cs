using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.resourceAggregates;
using RunnerWorld;

namespace RunnerWorld;

public class RunnerWorldImpl : IRunnerWorld
{
    public static void Run()
    {
        var il1 = new List<IToTestImports.Thing>();
        il1.Add(new IToTestImports.Thing(9));
        il1.Add(new IToTestImports.Thing(10));
        var il2 = new List<IToTestImports.Thing>();
        il2.Add(new IToTestImports.Thing(11));
        il2.Add(new IToTestImports.Thing(12));

        uint res = IToTestImports.Foo(
          new IToTestImports.R1(new IToTestImports.Thing(0)),
          new IToTestImports.R2(new IToTestImports.Thing(1)),
          new IToTestImports.R3(new IToTestImports.Thing(2), new IToTestImports.Thing(3)),
          (new IToTestImports.Thing(4), new IToTestImports.R1(new IToTestImports.Thing(5))),
          new IToTestImports.Thing(6),
          IToTestImports.V1.Thing(new IToTestImports.Thing(7)),
          IToTestImports.V2.Thing(new IToTestImports.Thing(8)),
          il1,
          il2,
          new IToTestImports.Thing(13),
          new IToTestImports.Thing(14),
    	  Result<IToTestImports.Thing, None>.Ok(new IToTestImports  .Thing(15)),
    	  Result<IToTestImports.Thing, None>.Ok(new IToTestImports.Thing(16))
        );
        Debug.Assert(res == 156);
    }
}
