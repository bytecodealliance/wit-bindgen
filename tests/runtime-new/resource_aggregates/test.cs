namespace TestWorld.wit.exports.test.resourceAggregates
{
    public class ToTestImpl : IToTest {
	public class ThingResource : IToTest.ThingResource, IToTest.IThingResource {
	    public uint val;

	    public ThingResource(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(
	    IToTest.R1 r1,
	    IToTest.R2 r2,
	    IToTest.R3 r3,
	    (IToTest.ThingResource, IToTest.R1) t1,
	    IToTest.ThingResource t2,
	    IToTest.V1 v1,
	    IToTest.V2 v2,
	    List<IToTest.ThingResource> l1,
	    List<IToTest.ThingResource> l2,
	    IToTest.ThingResource? o1,
	    IToTest.ThingResource? o2,
	    Result<IToTest.ThingResource, None> result1,
	    Result<IToTest.ThingResource, None> result2
	)
	{
            uint sumIl1 = 0;
            uint sumIl2 = 0;
	    foreach (var thing in l1)
	    {
		sumIl1 += ((ThingResource) thing).val;
	    }
	    foreach (var thing in l2)
	    {
		sumIl2 += ((ThingResource) thing).val;
	    }
            return ((ThingResource) r1.thing).val +
                   ((ThingResource) r2.thing).val +
                   ((ThingResource) r3.thing1).val +
                   ((ThingResource) r3.thing2).val +
                   ((ThingResource) t1.Item1).val +
                   ((ThingResource) t1.Item2.thing).val +
                   ((ThingResource) t2).val +
                   ((ThingResource) v1.AsThing).val +
                   ((ThingResource) v2.AsThing).val +
                   sumIl1 +
                   sumIl2 +
                   ((ThingResource) o1).val +
                   ((ThingResource) o2).val +
		   ((ThingResource) result1.AsOk).val +
		   ((ThingResource) result2.AsOk).val +
                   3;
	}
    }
}
