using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.resourceAggregates;
using RunnerWorld;

public class Program {
    public static void Main()
    {
        var il1 = new List<IToTest.Thing>();
        il1.Add(new IToTest.Thing(9));
        il1.Add(new IToTest.Thing(10));
        var il2 = new List<IToTest.Thing>();
        il2.Add(new IToTest.Thing(11));
        il2.Add(new IToTest.Thing(12));

        uint res = ToTestInterop.Foo(
          new IToTest.R1(new IToTest.Thing(0)),
          new IToTest.R2(new IToTest.Thing(1)),
          new IToTest.R3(new IToTest.Thing(2), new IToTest.Thing(3)),
          (new IToTest.Thing(4), new IToTest.R1(new IToTest.Thing(5))),
          new IToTest.Thing(6),
          IToTest.V1.Thing(new IToTest.Thing(7)),
          IToTest.V2.Thing(new IToTest.Thing(8)),
          il1,
          il2,
          new IToTest.Thing(13),
          new IToTest.Thing(14),
    	  Result<IToTest.Thing, None>.Ok(new IToTest.Thing(15)),
    	  Result<IToTest.Thing, None>.Ok(new IToTest.Thing(16))
        );
        Debug.Assert(res == 156);
    }
}
