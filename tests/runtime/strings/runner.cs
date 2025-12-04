using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.strings;
using System.Text;

public class Program 
{
    public static void Main(string[] args){
        IToTestImports.TakeBasic("latin utf16");
        Debug.Assert(IToTestImports.ReturnUnicode() == "🚀🚀🚀 𠈄𓀀");

        Debug.Assert(IToTestImports.ReturnEmpty() == string.Empty);
        Debug.Assert(IToTestImports.Roundtrip("🚀🚀🚀 𠈄𓀀") == "🚀🚀🚀 𠈄𓀀");
    }
}
