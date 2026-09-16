using System.Diagnostics;
using RunnerWorld.wit.Imports.my.test;
using RunnerWorld;

public class RunnerWorldExportsImpl
{
    public static async Task Run()
    {
        {
            var (reader, writer) = IIImports.FutureNewUint();
            await IIImports.CancelBeforeRead(reader);
            writer.Dispose();
        }

        
        {
            var (reader, writer) = IIImports.FutureNewUint();
            await IIImports.CancelAfterRead(reader);
            writer.Dispose();
        }

        {
            var (dataReader, dataWriter) = IIImports.FutureNewUint();
            var (signalReader, signalWriter) = IIImports.FutureNew();
            var testTask = IIImports.StartReadThenCancel(dataReader, signalReader);
            async Task WriterAsync()
            {
                // Make the data read ready first so that completing the signal
                // synchronously cancels the last other operation.
                await dataWriter.Write(4);
                await signalWriter.Write();
            }

            await WriterAsync();
            await testTask;
            dataWriter.Dispose();
            signalWriter.Dispose();
        }
    }
}
