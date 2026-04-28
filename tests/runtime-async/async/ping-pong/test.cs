using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static async Task<FutureReader<string>> Ping(FutureReader<string> future, string s)
        {
            Console.WriteLine("test: Ping started");
            var msg = (await future.Read()) + s;
            Console.WriteLine("test: Ping Read completed " + msg);
            var (newFutureReader, newFutureWriter) = IIExports.FutureNewString();
            Console.WriteLine("test: Ping return future creates");
            var writeTask = newFutureWriter.Write(msg);
            Console.WriteLine("test: Ping return future write started");
            writeTask.ContinueWith(t =>
            {
                if(t.Exception != null)
                {
                    Debug.Fail("Exception in returned future write." + t.Exception);                    
                }
            });
            return newFutureReader;
        }

        public static async Task<string> Pong(FutureReader<string> future)
        {
            return await future.Read();
        }
    }
}
