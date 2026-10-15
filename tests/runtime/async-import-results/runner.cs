//@ wasmtime-flags = '-Wcomponent-model-async'

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;
using RunnerWorld;
using RunnerWorld.wit.Imports.my.test;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        var echoed = await IIImports.Echo("hi");
        Debug.Assert(echoed == "hi!");

        var p = await IIImports.MakePoint(3, 4);
        Debug.Assert(p.x == 3 && p.y == 4 && p.label == "(3, 4)");

        // Read with room for every element, so one read can lift several values.
        string[] all = ["a", "bb", "ccc"];
        var stream = await IIImports.Words(new List<string>(all));
        var received = new List<string>();
        var buffer = new string[all.Length];
        while (received.Count < all.Length)
        {
            var read = await stream.Read(buffer);
            Debug.Assert(read > 0);
            for (var i = 0; i < read; i++)
            {
                received.Add(buffer[i]);
            }
        }
        Debug.Assert(string.Join(",", received) == "a,bb,ccc");
        stream.Dispose();
    }

    public static int RunCallback()
    {
        throw new NotImplementedException();
    }
}
