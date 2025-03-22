namespace ResourceBorrowExportWorld.wit.exports.test.resourceBorrowExport
{
    public class TestImpl : ITest {
	public class ThingResource : ITest.ThingResource, ITest.IThingResource {
	    public uint val;

	    public ThingResource(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(ITest.ThingResource v) {
	    return ((ThingResource) v).val + 2;
	}
    }
}
