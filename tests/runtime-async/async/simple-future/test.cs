using System.Diagnostics;

namespace TestWorld.wit.exports.my.test
{
    public class IImpl : II
    {
        public static Task ReadFuture(FutureReader reader)
        {
            var task = reader.Read();

            Debug.Assert(task.IsCompleted);
            // TODO: Should we check the Count?

            reader.Dispose();

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
