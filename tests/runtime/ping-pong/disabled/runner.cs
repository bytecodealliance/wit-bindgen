using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        try
        {
            string pingResult;
            {
                var (reader, writer) = IIImports.FutureNewString();
                var pingTask = IIImports.Ping(reader, "world");
                await writer.Write("hello");
                var pingFutureResult = await pingTask;
                var result = await pingFutureResult.Read();
                Debug.Assert(result == "helloworld");

                pingResult = result;
            }

            {
                var (reader, writer) = IIImports.FutureNewString();
                var pongTask = IIImports.Pong(reader);
                await writer.Write(pingResult);
                var pongResult = await pongTask;
                Debug.Assert(pongResult == "helloworld");
            }
        }
        catch(Exception e)
        {
            Console.WriteLine(e);
        }
    }
}
