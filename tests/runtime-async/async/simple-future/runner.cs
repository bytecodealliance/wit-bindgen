using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.my.test;
using System.Text;

public class Program
{
    public static async Task Main(string[] args)
    {
        var (reader, writer) = II.ReadFutureNew();

        var task = II.ReadFuture(reader);

        Debug.Assert(task.IsCompleted);
    }
}
