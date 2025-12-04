namespace TestWorld.wit.Exports.test.resourceAggregates
{
    public class ToTestExportsImpl : IToTestExports {
	public class Thing : IToTestExports.Thing, IToTestExports.IThing {
	    public uint val;

	    public Thing(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(
	    IToTestExports.R1 r1,
	    IToTestExports.R2 r2,
	    IToTestExports.R3 r3,
	    (IToTestExports.Thing, IToTestExports.R1) t1,
	    IToTestExports.Thing t2,
	    IToTestExports.V1 v1,
	    IToTestExports.V2 v2,
	    List<IToTestExports.Thing> l1,
	    List<IToTestExports.Thing> l2,
	    IToTestExports.Thing? o1,
	    IToTestExports.Thing? o2,
	    Result<IToTestExports.Thing, None> result1,
	    Result<IToTestExports.Thing, None> result2
	)
	{
            uint sumIl1 = 0;
            uint sumIl2 = 0;
	    foreach (var thing in l1)
	    {
		sumIl1 += ((Thing) thing).val;
	    }
	    foreach (var thing in l2)
	    {
		sumIl2 += ((Thing) thing).val;
	    }
            return ((Thing) r1.thing).val +
                   ((Thing) r2.thing).val +
                   ((Thing) r3.thing1).val +
                   ((Thing) r3.thing2).val +
                   ((Thing) t1.Item1).val +
                   ((Thing) t1.Item2.thing).val +
                   ((Thing) t2).val +
                   ((Thing) v1.AsThing).val +
                   ((Thing) v2.AsThing).val +
                   sumIl1 +
                   sumIl2 +
                   ((Thing) o1).val +
                   ((Thing) o2).val +
		   ((Thing) result1.AsOk).val +
		   ((Thing) result2.AsOk).val +
                   3;
	}
    }
}
