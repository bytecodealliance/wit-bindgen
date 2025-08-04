using System.Diagnostics;
using TestWorld.wit.exports.a.b;

namespace TestWorld.wit.exports.a.b
{
    public class IImpl : II
    {
        public static async Task OneArgument(uint x)
        {
            Debug.Assert(x == 1);
        }

        public static async Task<uint> OneResult()
        {
            return 2;
        }

        public static async Task<uint> OneArgumentAndResult(uint x)
        {
            Debug.Assert(x == 3);
            return 4;
        }

        public static async Task TwoArguments(uint x, uint y)
        {
            Debug.Assert(x == 5);
            Debug.Assert(y == 6);
        }

        public static async Task<uint> TwoArgumentsAndResult(uint x, uint y)
        {
            Debug.Assert(x == 7);
            Debug.Assert(y == 8);
            return 9;
        }
    }
}
