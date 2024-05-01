using System.Diagnostics;

namespace ResourceIntoInnerWorld.wit.exports.test.resourceIntoInner
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public string val;

	    public Thing(string v) {
		this.val = v;
	    }
	}

	public static void Test() {
	    // Unlike wasm.rs, this doesn't test anything interesting
	    // due there being no ownership semantics in C# (and also
	    // due to way the C# code generator lazily calls
	    // `[resource-new]thing` only as needed), but we go
	    // through the motions here anyway:
	    var text = "Jabberwocky";
	    Debug.Assert(new Thing(text).val == text);
	}
    }
}
