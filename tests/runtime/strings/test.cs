using System.Diagnostics;

namespace TestWorld.wit.Exports.test.strings
{
    public class ToTestExportsImpl : IToTestExports
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
