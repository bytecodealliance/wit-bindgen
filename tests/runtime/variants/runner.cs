using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.variants;
using System.Text;
using RunnerWorld;

namespace RunnerWorld;

public class RunnerWorldImpl : IRunnerWorld
{
    public static void Run()
    {
        Debug.Assert(IToTestImports.RoundtripOption(1.0f).Value == 1);
        Debug.Assert(IToTestImports.RoundtripOption(null).HasValue == false);
        Debug.Assert(IToTestImports.RoundtripOption(2.0f).Value == 2);

        Debug.Assert(IToTestImports.RoundtripResult(Result<uint, float>.Ok(2)) == 2.0);
        Debug.Assert(IToTestImports.RoundtripResult(Result<uint, float>.Ok(4)) == 4.0);
        try
        {
            IToTestImports.RoundtripResult(Result<uint, float>.Err(5.3f));
            throw new Exception();
        }
        catch (WitException e)
        {
            Debug.Assert((byte)e.Value == 5);
        }

        Debug.Assert(IToTestImports.RoundtripEnum(IToTestImports.E1.A) == IToTestImports.E1.A);
        Debug.Assert(IToTestImports.RoundtripEnum(IToTestImports.E1.B) == IToTestImports.E1.B);

        Debug.Assert(IToTestImports.InvertBool(true) == false);
        Debug.Assert(IToTestImports.InvertBool(false) == true);
        var (a1, a2, a3, a4, a5, a6) =
        IToTestImports.VariantCasts((IToTestImports.C1.A(1), IToTestImports.C2.A(2), IToTestImports.C3.A(3), IToTestImports.C4.A(4), IToTestImports.C5.A(5), IToTestImports.C6.A(6.0f)));
        Debug.Assert(a1.AsA == 1);
        Debug.Assert(a2.AsA == 2);
        Debug.Assert(a3.AsA == 3);
        Debug.Assert(a4.AsA == 4);
        Debug.Assert(a5.AsA == 5);
        Debug.Assert(a6.AsA == 6.0f);

        var (b1, b2, b3, b4, b5, b6) =
IToTestImports.VariantCasts((IToTestImports.C1.B(1), IToTestImports.C2.B(2), IToTestImports.C3.B(3), IToTestImports.C4.B(4), IToTestImports.C5.B(5), IToTestImports.C6.B(6.0)));
        Debug.Assert(b1.AsB == 1);
        Debug.Assert(b2.AsB == 2.0f);
        Debug.Assert(b3.AsB == 3.0f);
        Debug.Assert(b4.AsB == 4.0f);
        Debug.Assert(b5.AsB == 5.0f);
        Debug.Assert(b6.AsB == 6.0);

        var (za1, za2, za3, za4) =
IToTestImports.VariantZeros((IToTestImports.Z1.A(1), IToTestImports.Z2.A(2), IToTestImports.Z3.A(3.0f), IToTestImports.Z4.A(4.0f)));
        Debug.Assert(za1.AsA == 1);
        Debug.Assert(za2.AsA == 2);
        Debug.Assert(za3.AsA == 3.0f);
        Debug.Assert(za4.AsA == 4.0f);

        var (zb1, zb2, zb3, zb4) =
IToTestImports.VariantZeros((IToTestImports.Z1.B(), IToTestImports.Z2.B(), IToTestImports.Z3.B(), IToTestImports.Z4.B()));
        //TODO: Add comparison operator to variants and None
        //Debug.Assert(zb1.AsB == IToTest.Z1.b());
        //Debug.Assert(zb2.AsB == IToTest.Z2.b());
        //Debug.Assert(zb3.AsB == IToTest.Z3.b());
        //Debug.Assert(zb4.AsB == IToTest.Z4.b());

        IToTestImports.VariantTypedefs(null, false, Result<uint, None>.Err(new None()));

        var (a, b, c) = IToTestImports.VariantEnums(true, Result<None, None>.Ok(new None()), IToTestImports.MyErrno.SUCCESS);
        Debug.Assert(a == true);
        var test = b.AsOk;
        Debug.Assert(c == IToTestImports.MyErrno.SUCCESS);
    }
}
