using Import1 = IntermediateWorld.wit.Imports.IImportsImports;
using Import2 = IntermediateWorld.wit.Imports.test.resourceFloats.ITestImports;

namespace IntermediateWorld.wit.Exports
{
    public class ExportsExportsImpl : IExportsExports {
	public class Float : IExportsExports.Float, IExportsExports.IFloat {
	    public Import1.Float val;

	    public Float(double v) {
		    this.val = new Import1.Float(v + 1.0);
	    }

	    public double Get() {
		    return this.val.Get() + 3.0;
	    }

	    public static IExportsExports.Float Add(IExportsExports.Float a, double b) {
            return new Float(Import1.Float.Add(((Float) a).val, b).Get() + 5.0);
	    }
	}
    }
}

namespace IntermediateWorld {
    public class IntermediateWorldExportsImpl : Import2 {
	public static Import2.Float Add(Import2.Float a, Import2.Float b) {
	    return new Import2.Float(a.Get() + b.Get() + 5.0);
	}
    }
}
