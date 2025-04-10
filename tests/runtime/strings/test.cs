using System;
using System.Runtime.InteropServices;
using System.Diagnostics;

namespace TestWorld.wit.exports.test.strings
{
    public class ToTestImpl : ITestWorld
    {
        public static void TakeBasic(string s)
        {
            Debug.Assert(s == "latin utf16");
        }

        public static string ReturnUnicode() 
        {
            return "🚀🚀🚀 𠈄𓀀";
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
}
