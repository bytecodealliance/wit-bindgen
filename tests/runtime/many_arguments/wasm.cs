using System.Diagnostics;
using ManyArgumentsWorld;
using ManyArgumentsWorld.wit.imports;

namespace ManyArgumentsWorld;

public class ManyArgumentsWorldImpl : IManyArgumentsWorld
{
    public static void ManyArguments(
        ulong a1,
        ulong a2,
        ulong a3,
        ulong a4,
        ulong a5,
        ulong a6,
        ulong a7,
        ulong a8,
        ulong a9,
        ulong a10,
        ulong a11,
        ulong a12,
        ulong a13,
        ulong a14,
        ulong a15,
        ulong a16)
    {
        Debug.Assert(a1 == 1);
        Debug.Assert(a2 == 2);
        Debug.Assert(a3 == 3);
        Debug.Assert(a4 == 4);
        Debug.Assert(a5 == 5);
        Debug.Assert(a6 == 6);
        Debug.Assert(a7 == 7);
        Debug.Assert(a8 == 8);
        Debug.Assert(a9 == 9);
        Debug.Assert(a10 == 10);
        Debug.Assert(a11 == 11);
        Debug.Assert(a12 == 12);
        Debug.Assert(a13 == 13);
        Debug.Assert(a14 == 14);
        Debug.Assert(a15 == 15);
        Debug.Assert(a16 == 16);

        ImportsInterop.ManyArguments(
            a1,
            a2,
            a3,
            a4,
            a5,
            a6,
            a7,
            a8,
            a9,
            a10,
            a11,
            a12,
            a13,
            a14,
            a15,
            a16
        );
    }
}
