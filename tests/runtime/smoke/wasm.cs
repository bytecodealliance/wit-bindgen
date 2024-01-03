using System;
using System.Diagnostics;
using wit_smoke.wit.imports.test.smoke.Imports;

namespace wit_smoke;

public class SmokeWorldImpl : ISmokeWorld
{
    public static void Thunk()
    {
        ImportsInterop.Thunk();
    }
}
