using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.resourceAggregates;
using RunnerWorld;

public class Program {
    public static void Main()
    {
        var il1 = new List<IToTest.ThingResource>();
        il1.Add(new IToTest.ThingResource(9));
        il1.Add(new IToTest.ThingResource(10));
        var il2 = new List<IToTest.ThingResource>();
        il2.Add(new IToTest.ThingResource(11));
        il2.Add(new IToTest.ThingResource(12));

        uint res = ToTestInterop.Foo(
          new IToTest.R1(new IToTest.ThingResource(0)),
          new IToTest.R2(new IToTest.ThingResource(1)),
          new IToTest.R3(new IToTest.ThingResource(2), new IToTest.ThingResource(3)),
          (new IToTest.ThingResource(4), new IToTest.R1(new IToTest.ThingResource(5))),
          new IToTest.ThingResource(6),
          IToTest.V1.Thing(new IToTest.ThingResource(7)),
          IToTest.V2.Thing(new IToTest.ThingResource(8)),
          il1,
          il2,
          new IToTest.ThingResource(13),
          new IToTest.ThingResource(14),
    	  Result<IToTest.ThingResource, None>.Ok(new IToTest.ThingResource(15)),
    	  Result<IToTest.ThingResource, None>.Ok(new IToTest.ThingResource(16))
        );
        Debug.Assert(res == 156);
    }
}
