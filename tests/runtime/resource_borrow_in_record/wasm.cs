using Import = ResourceBorrowInRecordWorld.wit.imports.test.resourceBorrowInRecord.ITest;
using Host = ResourceBorrowInRecordWorld.wit.imports.test.resourceBorrowInRecord.TestInterop;

namespace ResourceBorrowInRecordWorld.wit.exports.test.resourceBorrowInRecord
{
    public class TestImpl : ITest {
	public class ThingResource : ITest.ThingResource, ITest.IThingResource {
	    public Import.ThingResource val;

	    public ThingResource(string v) {
		this.val = new Import.ThingResource(v + " Thing");
	    }

	    public ThingResource(Import.ThingResource thing) {
		this.val = thing;
	    }

	    public string Get() {
		return val.Get() + " Thing.get";
	    }
	}

	public static List<ITest.ThingResource> Test(List<ITest.Foo> v) {
	    var list = new List<Import.Foo>();
	    foreach (var foo in v)
	    {
		list.Add(new Import.Foo(((ThingResource) foo.thing).val));
	    }
	    var result = Host.Test(list);
	    var myResult = new List<ITest.ThingResource>();
	    foreach (var thing in result)
	    {
		myResult.Add(new ThingResource(thing));
	    }
	    return myResult;
	}
    }
}
