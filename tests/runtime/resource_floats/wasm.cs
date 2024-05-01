using Import1 = ResourceFloatsWorld.wit.imports.IImports;
using Import2 = ResourceFloatsWorld.wit.imports.test.resourceFloats.ITest;

namespace ResourceFloatsWorld.wit.exports
{
    public class ExportsImpl : IExports {
	public class Float : IExports.Float, IExports.IFloat {
	    public Import1.Float val;

	    public Float(double v) {
		this.val = new Import1.Float(v + 1.0);
	    }

	    public double Get() {
		return this.val.Get() + 3.0;
	    }

	    public static IExports.Float Add(IExports.Float a, double b) {
		return new Float(Import1.Float.Add(((Float) a).val, b).Get() + 5.0);
	    }
	}
    }
}

namespace ResourceFloatsWorld {
    public class ResourceFloatsWorldImpl : IResourceFloatsWorld {
	public static Import2.Float Add(Import2.Float a, Import2.Float b) {
	    return new Import2.Float(a.Get() + b.Get() + 5.0);
	}
    }
}
