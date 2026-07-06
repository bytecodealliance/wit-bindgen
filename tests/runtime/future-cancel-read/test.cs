using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static Task CancelBeforeRead(FutureReader<uint> future)
        {
            Debug.Assert(future.Read().Cancel() == CancelCode.Cancelled);
            future.Dispose();
            return Task.CompletedTask;
        }

        public static Task CancelAfterRead(FutureReader<uint> future)
        {
            var task = future.Read();
            Debug.Assert(!task.IsCompleted);

            // If the cancel occurs before the read is complete (or the writer ignores the cancel) we return Cancelled.
            Debug.Assert(task.Cancel() == CancelCode.Cancelled);
            return Task.CompletedTask;
        }

        public static async Task StartReadThenCancel(FutureReader<uint> future, FutureReader signal)
        {
            var task = future.Read();
            Debug.Assert(!task.IsCompleted);

            await signal.Read();

            Debug.Assert(task.Cancel() == CancelCode.Completed);
        }
    }
}