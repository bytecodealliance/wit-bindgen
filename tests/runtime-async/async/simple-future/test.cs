using System.Diagnostics;

namespace TestWorld.wit.exports.my.test
{
    public class IImpl : II
    {
        public static Task ReadFuture(FutureReader reader)
        {
            return Task.CompletedTask;
        }

        public static Task DropFuture(FutureReader reader)
        {
            return Task.CompletedTask;
        }
    }
}
