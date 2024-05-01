using Import = ResourceAggregatesWorld.wit.imports.test.resourceAggregates.ITest;
using Host = ResourceAggregatesWorld.wit.imports.test.resourceAggregates.TestInterop;

namespace ResourceAggregatesWorld.wit.exports.test.resourceAggregates
{
    public class TestImpl : ITest {
	public class Thing : ITest.Thing, ITest.IThing {
	    public Import.Thing val;

	    public Thing(uint v) {
		this.val = new Import.Thing(v + 1);
	    }
	}

	public static uint Foo(
	    ITest.R1 r1,
	    ITest.R2 r2,
	    ITest.R3 r3,
	    (ITest.Thing, ITest.R1) t1,
	    ITest.Thing t2,
	    ITest.V1 v1,
	    ITest.V2 v2,
	    List<ITest.Thing> l1,
	    List<ITest.Thing> l2,
	    Option<ITest.Thing> o1,
	    Option<ITest.Thing> o2,
	    Result<ITest.Thing, None> result1,
	    Result<ITest.Thing, None> result2
	)
	{
	    var ir1 = new Import.R1(((Thing) r1.thing).val);
	    var ir2 = new Import.R2(((Thing) r2.thing).val);
	    var ir3 = new Import.R3(((Thing) r3.thing1).val, ((Thing) r3.thing2).val);
	    var it1 = (((Thing) t1.Item1).val, new Import.R1(((Thing) t1.Item2.thing).val));
	    var it2 = ((Thing) t2).val;
	    var iv1 = Import.V1.thing(((Thing) v1.AsThing).val);
	    var iv2 = Import.V2.thing(((Thing) v2.AsThing).val);
	    var il1 = new List<Import.Thing>();
	    foreach (var thing in l1)
	    {
		il1.Add(((Thing) thing).val);
	    }
	    var il2 = new List<Import.Thing>();
	    foreach (var thing in l2)
	    {
		il2.Add(((Thing) thing).val);
	    }
	    var io1 = o1.HasValue
		? new Option<Import.Thing>(((Thing) o1.Value).val)
		: Option<Import.Thing>.None;
	    var io2 = o2.HasValue
		? new Option<Import.Thing>(((Thing) o2.Value).val)
		: Option<Import.Thing>.None;
	    var iresult1 = result1.IsOk
		? Result<Import.Thing, None>.ok(((Thing) result1.AsOk).val)
		: Result<Import.Thing, None>.err(new None());
	    var iresult2 = result2.IsOk
		? Result<Import.Thing, None>.ok(((Thing) result2.AsOk).val)
		: Result<Import.Thing, None>.err(new None());

	    return Host.Foo(ir1, ir2, ir3, it1, it2, iv1, iv2, il1, il2, io1, io2, iresult1, iresult2) + 4;
	}
    }
}
