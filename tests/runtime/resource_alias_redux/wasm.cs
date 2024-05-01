using Import1 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.IResourceAlias1;
using Import2 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.IResourceAlias2;
using Host1 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.ResourceAlias1Interop;
using Host2 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.ResourceAlias2Interop;

namespace ResourceAliasReduxWorld.wit.exports.test.resourceAliasRedux
{
    public class ResourceAlias1Impl : IResourceAlias1 {
	public class Thing : IResourceAlias1.Thing, IResourceAlias1.IThing {
	    public Import1.Thing val;

	    public Thing(string v) {
		this.val = new Import1.Thing(v + " Thing");
	    }

	    public Thing(Import1.Thing v) {
		this.val = v;
	    }

	    public string Get() {
		return this.val.Get() + " Thing.get";
	    }
	}

	public static List<IResourceAlias1.Thing> A(IResourceAlias1.Foo f) {
	    var oldList = Host1.A(new Import1.Foo(((Thing) f.thing).val));
	    var newList = new List<IResourceAlias1.Thing>();
	    foreach (var thing in oldList)
	    {
		newList.Add(new Thing(thing));
	    }
	    return newList;
	}
    }
    
    public class ResourceAlias2Impl : IResourceAlias2 {
	public static List<IResourceAlias1.Thing> B(IResourceAlias2.Foo f, IResourceAlias1.Foo g) {
	    var oldList = Host2.B(
		new Import2.Foo(((ResourceAlias1Impl.Thing) f.thing).val),
		new Import1.Foo(((ResourceAlias1Impl.Thing) g.thing).val)
	    );
	    var newList = new List<IResourceAlias1.Thing>();
	    foreach (var thing in oldList)
	    {
		newList.Add(new ResourceAlias1Impl.Thing(thing));
	    }
	    return newList;
	}
    }
}

namespace ResourceAliasReduxWorld {
    public class ResourceAliasReduxWorldImpl : IResourceAliasReduxWorld {
	public static List<Import1.Thing> Test(List<Import1.Thing> things) {
	    return things;
	}
    }
}
