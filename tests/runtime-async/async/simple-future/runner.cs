using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.my.test;
using RunnerWorld;

public class Program
{
    public static async Task Main(string[] args)
    {
        {
            var (reader, writer) = II.ReadFutureVoidNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = II.ReadFuture(reader);
            Debug.Assert(task.IsCompleted);

            var set = II.WaitableSetNew();
            II.Join(writer, set);

            var ev = new EventWaitable();
            II.WaitableSetWait(set, ref ev);
            //      assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
            // assert(event.waitable == writer);
            // assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_COMPLETED);
            // assert(RUNNER_WAITABLE_COUNT(event.code) == 0);
        }
    }
}