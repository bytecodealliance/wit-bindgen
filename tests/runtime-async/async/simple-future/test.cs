using System;
using System.Diagnostics;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static Task ReadFuture(FutureReader reader)
        {
            var task = reader.Read();

            Debug.Assert(task.IsCompleted);
            // TODO: Should we check the Count?

            reader.Dispose();

            Console.WriteLine("ReadFuture finished");
            return Task.CompletedTask;
        }

        public static int ReadFutureCallback()
        {
            Debug.Assert(false);
            return 0;
        }

        public static Task DropFuture(FutureReader reader)
        {
            reader.Dispose();
            return Task.CompletedTask;
        }

        public static int DropFutureCallback()
        {
            Debug.Assert(false);
            return 0;
        }

    }
}
