namespace TestWorld.wit.exports.test.resourceAggregates
{
    public class ToTestImpl : IToTest {
	public class Thing : IToTest.Thing, IToTest.IThing {
	    public uint val;

	    public Thing(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(
	    IToTest.R1 r1,
	    IToTest.R2 r2,
	    IToTest.R3 r3,
	    (IToTest.Thing, IToTest.R1) t1,
	    IToTest.Thing t2,
	    IToTest.V1 v1,
	    IToTest.V2 v2,
	    List<IToTest.Thing> l1,
	    List<IToTest.Thing> l2,
	    IToTest.Thing? o1,
	    IToTest.Thing? o2,
	    Result<IToTest.Thing, None> result1,
	    Result<IToTest.Thing, None> result2
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
