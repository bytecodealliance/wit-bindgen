using System.Text;
using Import = ResourceWithListsWorld.wit.imports.test.resourceWithLists.ITest;
using Host = ResourceWithListsWorld.wit.imports.test.resourceWithLists.TestInterop;

namespace ResourceWithListsWorld.wit.exports.test.resourceWithLists
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public Import.Thing val;

	    public Thing(byte[] v) {
		var bytes = Encoding.ASCII.GetBytes(" Thing");
		var result = new byte[v.Count() + bytes.Count()];
		Array.Copy(v, result, v.Count());
		Array.Copy(bytes, 0, result, v.Count(), bytes.Count());
		this.val = new Import.Thing(result);
	    }

	    public byte[] Foo() {
		var v = this.val.Foo();
		var bytes = Encoding.ASCII.GetBytes(" Thing.foo");
		var result = new byte[v.Count() + bytes.Count()];
		Array.Copy(v, result, v.Count());
		Array.Copy(bytes, 0, result, v.Count(), bytes.Count());
		return result;
	    }

	    public void Bar(byte[] v) {
		var bytes = Encoding.ASCII.GetBytes(" Thing.bar");
		var result = new byte[v.Count() + bytes.Count()];
		Array.Copy(v, result, v.Count());
		Array.Copy(bytes, 0, result, v.Count(), bytes.Count());
		this.val.Bar(result);
	    }

	    public static byte[] Baz(byte[] v) {
		var bytes = Encoding.ASCII.GetBytes(" Thing.baz");
		var result = new byte[v.Count() + bytes.Count()];
		Array.Copy(v, result, v.Count());
		Array.Copy(bytes, 0, result, v.Count(), bytes.Count());

		var v2 = Import.Thing.Baz(result);
		var bytes2 = Encoding.ASCII.GetBytes(" Thing.baz again");
		var result2 = new byte[v2.Count() + bytes2.Count()];
		Array.Copy(v2, result2, v2.Count());
		Array.Copy(bytes2, 0, result2, v2.Count(), bytes2.Count());

		return result2;
	    }
	}
    }
}
