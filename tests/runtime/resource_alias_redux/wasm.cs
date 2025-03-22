using Import1 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.IResourceAlias1;
using Import2 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.IResourceAlias2;
using Host1 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.ResourceAlias1Interop;
using Host2 = ResourceAliasReduxWorld.wit.imports.test.resourceAliasRedux.ResourceAlias2Interop;

namespace ResourceAliasReduxWorld.wit.exports.test.resourceAliasRedux
{
    public class ResourceAlias1Impl : IResourceAlias1 {
	public class ThingResource : IResourceAlias1.ThingResource, IResourceAlias1.IThingResource {
	    public Import1.ThingResource val;

	    public ThingResource(string v) {
		this.val = new Import1.ThingResource(v + " Thing");
	    }

	    public ThingResource(Import1.ThingResource v) {
		this.val = v;
	    }

	    public string Get() {
		return this.val.Get() + " Thing.get";
	    }
	}

	public static List<IResourceAlias1.ThingResource> A(IResourceAlias1.Foo f) {
	    var oldList = Host1.A(new Import1.Foo(((ThingResource) f.thing).val));
	    var newList = new List<IResourceAlias1.ThingResource>();
	    foreach (var thing in oldList)
	    {
		newList.Add(new ThingResource(thing));
	    }
	    return newList;
	}
    }
    
    public class ResourceAlias2Impl : IResourceAlias2 {
	public static List<IResourceAlias1.ThingResource> B(IResourceAlias2.Foo f, IResourceAlias1.Foo g) {
	    var oldList = Host2.B(
		new Import2.Foo(((ResourceAlias1Impl.ThingResource) f.thing).val),
		new Import1.Foo(((ResourceAlias1Impl.ThingResource) g.thing).val)
	    );
	    var newList = new List<IResourceAlias1.ThingResource>();
	    foreach (var thing in oldList)
	    {
		newList.Add(new ResourceAlias1Impl.ThingResource(thing));
	    }
	    return newList;
	}
    }
}

namespace ResourceAliasReduxWorld {
    public class ResourceAliasReduxWorldImpl : IResourceAliasReduxWorld {
	public static List<Import1.ThingResource> Test(List<Import1.ThingResource> things) {
	    return things;
	}
    }
}
