using Import = ResourceImportAndExportWorld.wit.imports.test.resourceImportAndExport.ITest;
using Host = ResourceImportAndExportWorld.wit.imports.test.resourceImportAndExport.TestInterop;

namespace ResourceImportAndExportWorld.wit.exports.test.resourceImportAndExport
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public Import.Thing val;

	    public Thing(uint v) {
		this.val = new Import.Thing(v + 1);
	    }

	    public uint Foo() {
		return this.val.Foo() + 2;
	    }

	    public void Bar(uint v) {
		this.val.Bar(v + 3);
	    }

	    public static ITest.Thing Baz(ITest.Thing a, ITest.Thing b) {
		return new Thing(Import.Thing.Baz(((Thing) a).val, ((Thing) b).val).Foo() + 4);
	    }
	}
    }
}

namespace ResourceImportAndExportWorld {
    public class ResourceImportAndExportWorldImpl : IResourceImportAndExportWorld {
	public static Import.Thing ToplevelExport(Import.Thing things) {
	    return exports.ResourceImportAndExportWorld.ToplevelImport(things);
	}
    }
}
