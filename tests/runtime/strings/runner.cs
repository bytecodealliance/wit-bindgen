using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.strings;
using System.Text;

public class Program 
{
    public static void Main(string[] args){
        ToTestInterop.TakeBasic("latin utf16");
        Debug.Assert(ToTestInterop.ReturnUnicode() == "🚀🚀🚀 𠈄𓀀");

        Debug.Assert(ToTestInterop.ReturnEmpty() == string.Empty);
        Debug.Assert(ToTestInterop.Roundtrip("🚀🚀🚀 𠈄𓀀") == "🚀🚀🚀 𠈄𓀀");
    }
}
