using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static async Task<FutureReader<string>> Ping(FutureReader<string> future, string s)
        {
            var msg = (await future.Read()) + s;
            var (newFutureReader, newFutureWriter) = IIExports.FutureNewString();
            await newFutureWriter.Write(msg);

            return newFutureReader;
        }

        public static async Task<string> Pong(FutureReader<string> future)
        {
            return await future.Read();
        }
    }
}
