using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        var (reader, writer) = IIImports.FutureNew();

        var task = IIImports.PendingImport(reader);
        Debug.Assert(!task.IsCompleted);

        var writeTask = writer.Write();
        await task;
        Debug.Assert(!task.IsFaulted && task.IsCompleted);
        writer.Dispose();
        reader.Dispose();
    }
}