namespace TestWorld.wit.exports.test.resourceBorrow
{
    public class ToTestImpl : IToTest {
	public class Thing : IToTest.Thing, IToTest.IThing {
	    public uint val;

	    public Thing(uint v) {
		this.val = v + 1;
	    }
	}

	public static uint Foo(IToTest.Thing v) {
	    return ((Thing) v).val + 2;
	}
    }
}
