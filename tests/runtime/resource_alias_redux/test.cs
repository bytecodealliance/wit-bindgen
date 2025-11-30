using TestWorld.wit.Exports.test.resourceAliasRedux;

namespace TestWorld.wit.Exports
{
    public class ResourceAlias1ExportsImpl : IResourceAlias1Exports {
	public class Thing : IResourceAlias1Exports.Thing, IResourceAlias1Exports.IThing {
	    public string val;

	    public Thing(string v) {
		this.val = v + " GuestThing";
	    }

	    public string Get() {
		return this.val + " GuestThing.get";
	    }
	}

	public static List<IResourceAlias1Exports.Thing> A(IResourceAlias1Exports.Foo f) {
	    var newList = new List<IResourceAlias1Exports.Thing>();
	    newList.Add(f.thing);
	    return newList;
	}
    }

    public class ResourceAlias2ExportsImpl : IResourceAlias2Exports {
	public static List<IResourceAlias1Exports.Thing> B(IResourceAlias2Exports.Foo f, IResourceAlias1Exports.Foo g) {
	    var newList = new List<IResourceAlias1Exports.Thing>();
	    newList.Add(f.thing);
	    newList.Add(g.thing);
	    return newList;
	}
    }
}

namespace TestWorld {
	using TestWorld.wit.Exports.test.resourceAliasRedux;
    using TestWorld.wit.Exports;

    public class TheTestExportsImpl : ITheTestExports
    {
	public static List<IResourceAlias1Exports.Thing> Test(List<IResourceAlias1Exports.Thing> things) {
	    return things;
	}
    }
}
