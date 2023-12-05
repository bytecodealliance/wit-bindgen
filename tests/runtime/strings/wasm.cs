using System;
using System.Diagnostics;
using wit_strings.wit.imports.test.strings.Imports;

namespace wit_strings;

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