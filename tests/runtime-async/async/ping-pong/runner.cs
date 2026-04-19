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
            var pongResult = await pingTask;
            var result = await pongResult.Read();
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

        // let (tx, rx) = wit_future::new(|| unreachable!());
        // let f1 = async move {
        //     let m3 = pong(rx).await;
        //     assert_eq!(m3, "helloworld");
        // };
        // let f2 = async { tx.write(m2).await.unwrap() };
        // let ((), ()) = futures::join!(f1, f2);

    }
}
