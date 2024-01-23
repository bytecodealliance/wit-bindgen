using System;
using System.Diagnostics;
using SmokeWorld;
using SmokeWorld.wit.imports.test.smoke;

namespace SmokeWorld;

public class SmokeWorldImpl : ISmokeWorld
{
    public static void Thunk()
    {
        ImportsInterop.Thunk();
    }
}
