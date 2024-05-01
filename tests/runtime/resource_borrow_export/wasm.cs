namespace ResourceBorrowExportWorld.wit.exports.test.resourceBorrowExport
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public uint val;

	    public Thing(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(ITest.Thing v) {
	    return ((Thing) v).val + 2;
	}
    }
}
