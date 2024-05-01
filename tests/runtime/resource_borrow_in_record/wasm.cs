using Import = ResourceBorrowInRecordWorld.wit.imports.test.resourceBorrowInRecord.ITest;
using Host = ResourceBorrowInRecordWorld.wit.imports.test.resourceBorrowInRecord.TestInterop;

namespace ResourceBorrowInRecordWorld.wit.exports.test.resourceBorrowInRecord
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public Import.Thing val;

	    public Thing(string v) {
		this.val = new Import.Thing(v + " Thing");
	    }

	    public Thing(Import.Thing thing) {
		this.val = thing;
	    }

	    public string Get() {
		return val.Get() + " Thing.get";
	    }
	}

	public static List<ITest.Thing> Test(List<ITest.Foo> v) {
	    var list = new List<Import.Foo>();
	    foreach (var foo in v)
	    {
		list.Add(new Import.Foo(((Thing) foo.thing).val));
	    }
	    var result = Host.Test(list);
	    var myResult = new List<ITest.Thing>();
	    foreach (var thing in result)
	    {
		myResult.Add(new Thing(thing));
	    }
	    return myResult;
	}
    }
}
