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

        Console.WriteLine("Writing to future to complete pending import...");
        var writeTask = writer.Write();
        Console.WriteLine("WriteTask IsCompleted: " + writeTask.IsCompleted);
        await task;
        Console.WriteLine("RunnerWorld PendingImport task is completed");
        Debug.Assert(!task.IsFaulted && task.IsCompleted);
        writer.Dispose();
        reader.Dispose();
    }
}