//@ wasmtime-flags = '-Wcomponent-model-async'

using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld;
using RunnerWorld.wit.Imports.my.test;
using System.Text;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        Console.WriteLine("start");
        var (rx, tx) = IIImports.StreamNewByte();
        Console.WriteLine("start2");
        async Task Test()
        {
        Console.WriteLine("start Test");
            var writtenOne = await tx.Write([0]);
        Console.WriteLine("start Test2");
            Debug.Assert(writtenOne == 1);

            var writtenTwo = await tx.Write([1, 2]);
            Debug.Assert(writtenTwo == 2);
        Console.WriteLine("start Test3");

            var writtenTwoAgain = await tx.Write([3, 4]);
            Debug.Assert(writtenTwoAgain == 2);
        Console.WriteLine("start Test4");

            bool exceptionThrownAndCaught = false;
            try
            {
                var writtenToDropped = await tx.Write([0]);
            }
            catch(StreamDroppedException)
            {
        Console.WriteLine("StreamDroppedException");
                exceptionThrownAndCaught = true;
            }
            Debug.Assert(exceptionThrownAndCaught);
        Console.WriteLine("start Test End");
        }
        Console.WriteLine("start3");
        
        try
        {
            await Task.WhenAll(Test(), IIImports.ReadStream(rx));
        }
        catch(Exception e)
        {
            Console.Error.WriteLine("exception");
            Console.Error.WriteLine(e);
        }
        Console.WriteLine("Run Exit");
    }

    public static int RunCallback()
    {
        Console.WriteLine("RunCallback");
        throw new NotImplementedException();
    }
}
