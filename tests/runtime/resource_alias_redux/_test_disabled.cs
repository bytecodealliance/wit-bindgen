namespace TestWorld.wit.exports.test.resourceAliasRedux
{
    public class ResourceAlias1Impl : IResourceAlias1 {
	public class Thing : IResourceAlias1.Thing, IResourceAlias1.IThing {
	    public string val;

	    public Thing(string v) {
		this.val = v + " GuestThing";
	    }

	    public string Get() {
		return this.val + " GuestThing.get";
	    }
	}

	public static List<IResourceAlias1.Thing> A(IResourceAlias1.Foo f) {
	    var newList = new List<IResourceAlias1.Thing>();
	    newList.Add(f.thing);
	    return newList;
	}
    }

    public class ResourceAlias2Impl : IResourceAlias2 {
	public static List<IResourceAlias1.Thing> B(IResourceAlias2.Foo f, IResourceAlias1.Foo g) {
	    var newList = new List<IResourceAlias1.Thing>();
	    newList.Add(f.thing);
	    newList.Add(g.thing);
	    return newList;
	}
    }
}

namespace TestWorld {
    using TestWorld.wit.exports.test.resourceAliasRedux;

    public class TestWorldImpl : ITestWorld {
	public static List<IResourceAlias1.Thing> Test(List<IResourceAlias1.Thing> things) {
	    return things;
	}
    }
}
