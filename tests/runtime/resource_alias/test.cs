namespace TestWorld.wit.Exports.test.resourceAlias
{
    public class E1ExportsImpl : IE1Exports {
	public class X : IE1Exports.X, IE1Exports.IX {
	    public uint val;

	    public X(uint v) {
		this.val = v;
	    }
	}

	public static List<IE1Exports.X> A(IE1Exports.Foo f) {
	    return new List<IE1Exports.X>() { f.x };
	}
    }

    public class E2ExportsImpl : IE2Exports {
	public static List<IE1Exports.X> A(IE2Exports.Foo f, IE1Exports.Foo g, IE1Exports.X h) {
	    return new List<IE1Exports.X>() { f.x, g.x };
	}
    }
}
