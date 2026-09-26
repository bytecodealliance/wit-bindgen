using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        string pingResult;
        {
            var (reader, writer) = IIImports.FutureNewString();
            var pingTask = IIImports.Ping(reader, "world");
            await writer.Write("hello");
            writer.Dispose();
            var pingFutureResult = await pingTask;
            var result = await pingFutureResult.Read();
            pingFutureResult.Dispose();
            Debug.Assert(result == "helloworld");

            pingResult = result;
        }

        {
            var (reader, writer) = IIImports.FutureNewString();
            var pongTask = IIImports.Pong(reader);
            await writer.Write(pingResult);
            writer.Dispose();
            var pongResult = await pongTask;
            Debug.Assert(pongResult == "helloworld");
        }
    }
}
