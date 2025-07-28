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
    }
}
