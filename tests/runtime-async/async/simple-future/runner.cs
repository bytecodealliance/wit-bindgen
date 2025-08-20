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
            var status = II.WaitableSetWait(set);
            Debug.Assert(status.Event == EventCode.FutureWrite);
            Debug.Assert(status.Waitable == writer.Handle);
            Debug.Assert(status.Status.IsCompleted);
            Debug.Assert(status.Status.Count == 0);

            writer.Dispose();
            set.Dispose();
        }   

        {
            var (reader, writer) = II.DropFutureVoidNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = II.DropFuture(reader);
            Debug.Assert(task.IsCompleted);

            var set = II.WaitableSetNew();
            II.Join(writer, set);

            var ev = new EventWaitable();
            var status = II.WaitableSetWait(set);
            Debug.Assert(status.Event == EventCode.FutureWrite);
            Debug.Assert(status.Waitable == writer.Handle);
            Debug.Assert(status.Status.IsDropped);
            Debug.Assert(status.Status.Count == 0);

            writer.Dispose();
            set.Dispose();
        }
    }
}