//@ wasmtime-flags = '-Wcomponent-model-async'

using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        {
            var (reader, writer) = IIImports.FutureNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = IIImports.ReadFuture(reader);
            Debug.Assert(task.IsCompleted);
            await writeTask;

            writer.Dispose();
        }

        {
            var (reader, writer) = IIImports.FutureNew();

            var writeTask = writer.Write();
            Debug.Assert(!writeTask.IsCompleted);

            var task = IIImports.DropFuture(reader);
            Debug.Assert(task.IsCompleted);

            bool exceptionThrown = false;
            try
            {
                await writeTask;
            }
            catch(Exception)
            {
                exceptionThrown = true;
            }
            Debug.Assert(exceptionThrown);

            writer.Dispose();
        }
    }

    public static int RunCallback()
    {
        throw new NotImplementedException();
    }
}
