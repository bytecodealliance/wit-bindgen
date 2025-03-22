using Import1 = ResourceFloatsWorld.wit.imports.IImports;
using Import2 = ResourceFloatsWorld.wit.imports.test.resourceFloats.ITest;

namespace ResourceFloatsWorld.wit.exports
{
    public class ExportsImpl : IExports {
	public class FloatResource : IExports.FloatResource, IExports.IFloatResource {
	    public Import1.FloatResource val;

	    public FloatResource(double v) {
		this.val = new Import1.FloatResource(v + 1.0);
	    }

	    public double Get() {
		return this.val.Get() + 3.0;
	    }

	    public static IExports.FloatResource Add(IExports.FloatResource a, double b) {
		return new FloatResource(Import1.FloatResource.Add(((FloatResource) a).val, b).Get() + 5.0);
	    }
	}
    }
}

namespace ResourceFloatsWorld {
    public class ResourceFloatsWorldImpl : IResourceFloatsWorld {
	public static Import2.FloatResource Add(Import2.FloatResource a, Import2.FloatResource b) {
	    return new Import2.FloatResource(a.Get() + b.Get() + 5.0);
	}
    }
}
