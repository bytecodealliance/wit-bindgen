namespace TestWorld.wit.exports.test.resourceAlias
{
    public class E1Impl : IE1 {
	public class X : IE1.X, IE1.IX {
	    public uint val;

	    public X(uint v) {
		this.val = v;
	    }
	}

	public static List<IE1.X> A(IE1.Foo f) {
	    return new List<IE1.X>() { f.x };
	}
    }

    public class E2Impl : IE2 {
	public static List<IE1.X> A(IE2.Foo f, IE1.Foo g, IE1.X h) {
	    return new List<IE1.X>() { f.x, g.x };
	}
    }
}
