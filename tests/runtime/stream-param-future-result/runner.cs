//@ wasmtime-flags = '-Wcomponent-model-async'

using System;
using System.Diagnostics;
using System.Threading.Tasks;
using RunnerWorld;
using RunnerWorld.wit.Imports.my.test;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        var (rx, tx) = IIImports.StreamNewByte();
        var result = await IIImports.Consume(rx);
        var written = await tx.Write([1, 2, 3]);
        Debug.Assert(written == 3);
        tx.Dispose();
        var sum = await result.Read();
        Debug.Assert(sum.IsOk && sum.AsOk == 6);

        (rx, tx) = IIImports.StreamNewByte();
        result = await IIImports.Consume(rx);
        written = await tx.Write([0]);
        Debug.Assert(written == 1);
        tx.Dispose();
        var zero = await result.Read();
        Debug.Assert(zero.IsErr && zero.AsErr == "zero");
    }

    public static int RunCallback()
    {
        throw new NotImplementedException();
    }
}
