using IntermediateWorld.wit.Imports.test.resourceImportAndExport;
using Import = IntermediateWorld.wit.Imports.test.resourceImportAndExport.ITestImports;

namespace IntermediateWorld.wit.Exports.test.resourceImportAndExport
{
    public class TestExportsImpl : ITestExports {
	public class Thing : ITestExports.Thing, ITestExports.IThing {
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

	    public static ITestExports.Thing Baz(ITestExports.Thing a, ITestExports.Thing b) {
		return new Thing(Import.Thing.Baz(((Thing) a).val, ((Thing) b).val).Foo() + 4);
	    }
	}
    }
}

namespace IntermediateWorld {
    public class IntermediateWorldExportsImpl : ITestImports
    {
	public static Import.Thing ToplevelExport(Import.Thing things) {
	    return IntermediateWorld.IIntermediateWorldImports.ToplevelImport(things);
	}
    }
}
