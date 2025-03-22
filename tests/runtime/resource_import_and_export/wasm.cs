using Import = ResourceImportAndExportWorld.wit.imports.test.resourceImportAndExport.ITest;
using Host = ResourceImportAndExportWorld.wit.imports.test.resourceImportAndExport.TestInterop;

namespace ResourceImportAndExportWorld.wit.exports.test.resourceImportAndExport
{
    public class TestImpl : ITest {
	public class ThingResource : ITest.ThingResource, ITest.IThingResource {
	    public Import.ThingResource val;

	    public ThingResource(uint v) {
		this.val = new Import.ThingResource(v + 1);
	    }

	    public uint Foo() {
		return this.val.Foo() + 2;
	    }

	    public void Bar(uint v) {
		this.val.Bar(v + 3);
	    }

	    public static ITest.ThingResource Baz(ITest.ThingResource a, ITest.ThingResource b) {
		return new ThingResource(Import.ThingResource.Baz(((ThingResource) a).val, ((ThingResource) b).val).Foo() + 4);
	    }
	}
    }
}

namespace ResourceImportAndExportWorld {
    public class ResourceImportAndExportWorldImpl : IResourceImportAndExportWorld {
	public static Import.ThingResource ToplevelExport(Import.ThingResource things) {
	    return exports.ResourceImportAndExportWorld.ToplevelImport(things);
	}
    }
}
