using System.Diagnostics;
using System.Runtime.InteropServices;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static async Task PendingImport(FutureReader future)
        {
            Console.WriteLine("Test: Waiting on pending import future...");
            await future;
            Console.WriteLine("Test: Pending import future completed.");
        }
    }
}
