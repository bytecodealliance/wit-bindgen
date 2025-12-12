namespace TestWorld.wit.Exports.test.resourceBorrow
{
    public class ToTestExportsImpl : IToTestExports {
	public class Thing : IToTestExports.Thing, IToTestExports.IThing {
	    public uint val;

	    public Thing(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(IToTestExports.Thing v) {
	    return ((Thing) v).val + 2;
	}
    }
}
