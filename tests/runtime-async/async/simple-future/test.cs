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

//   exports_test_future_void_drop_readable(future);
            //   exports_test_async_read_future_return();
            //   return TEST_CALLBACK_CODE_EXIT;

            return Task.CompletedTask;
        }

        public static Task DropFuture(FutureReader reader)
        {
            return Task.CompletedTask;
        }
    }
}
