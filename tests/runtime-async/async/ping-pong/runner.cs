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
        Console.WriteLine("Runner ping started");
            var (reader, writer) = IIImports.FutureNewString();
            var pingTask = IIImports.Ping(reader, "world");
        Console.WriteLine("Runner ping called");
            await writer.Write("hello");
        Console.WriteLine("Runner ping write complete");
            var pingFutureResult = await pingTask;
        Console.WriteLine("Runner ping complete");
            var result = await pingFutureResult.Read();
        Console.WriteLine("Runner pingFutureResult read complete");
            Debug.Assert(result == "helloworld");

            pingResult = result;
        }
        Console.WriteLine("Runner ping finished");

        {
            var (reader, writer) = IIImports.FutureNewString();
            var pongTask = IIImports.Pong(reader);
            await writer.Write(pingResult);
            var pongResult = await pongTask;
            Debug.Assert(pongResult == "helloworld");
        }

        Console.WriteLine("Run finished");
        // let (tx, rx) = wit_future::new(|| unreachable!());
        // let f1 = async move {
        //     let m3 = pong(rx).await;
        //     assert_eq!(m3, "helloworld");
        // };
        // let f2 = async { tx.write(m2).await.unwrap() };
        // let ((), ()) = futures::join!(f1, f2);
        }
        catch(Exception e)
        {
            Console.WriteLine(e);
        }
    }
}
