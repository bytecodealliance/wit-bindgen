using System;
using System.Diagnostics;
using StringsWorld.wit.imports.test.strings;

namespace StringsWorld;

public class StringsWorldImpl : IStringsWorld
{
    public static void TestImports()
    {
        ImportsInterop.TakeBasic("latin utf16");
        Debug.Assert(ImportsInterop.ReturnUnicode() == "🚀🚀🚀 𠈄𓀀");
    }

    public static string ReturnEmpty()
    {
        return "";
    }

    public static string Roundtrip(string s)
    {
        return s;
    }
}