using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class Program
{
    public static async Task Main(string[] args)
    {
        {
            var (reader, writer) = IIImports.ReadFutureVoidNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = IIImports.ReadFuture(reader);
            Debug.Assert(task.IsCompleted);

            var set = IIImports.WaitableSetNew();
            IIImports.Join(writer, set);

            var ev = new EventWaitable();
            var status = IIImports.WaitableSetWait(set);
            Debug.Assert(status.Event == EventCode.FutureWrite);
            Debug.Assert(status.Waitable == writer.Handle);
            Debug.Assert(status.Status.IsCompleted);
            Debug.Assert(status.Status.Count == 0);

            writer.Dispose();
            set.Dispose();
        }   

        {
            var (reader, writer) = IIImports.DropFutureVoidNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = IIImports.DropFuture(reader);
            Debug.Assert(task.IsCompleted);

            var set = IIImports.WaitableSetNew();
            IIImports.Join(writer, set);

            var ev = new EventWaitable();
            var status = IIImports.WaitableSetWait(set);
            Debug.Assert(status.Event == EventCode.FutureWrite);
            Debug.Assert(status.Waitable == writer.Handle);
            Debug.Assert(status.Status.IsDropped);
            Debug.Assert(status.Status.Count == 0);

            writer.Dispose();
            set.Dispose();
        }
    }
}