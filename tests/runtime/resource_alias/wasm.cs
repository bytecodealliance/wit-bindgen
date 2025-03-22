namespace ResourceAliasWorld.wit.exports.test.resourceAlias
{
    public class E1Impl : IE1 {
	public class XResource : IE1.XResource, IE1.IXResource {
	    public uint val;

	    public XResource(uint v) {
		this.val = v;
	    }
	}

	public static List<IE1.XResource> A(IE1.Foo f) {
	    return new List<IE1.XResource>() { f.x };
	}
    }
    
    public class E2Impl : IE2 {
	public static List<IE1.XResource> A(IE2.Foo f, IE1.Foo g, IE1.XResource h) {
	    return new List<IE1.XResource>() { f.x, g.x };
	}
    }
}
