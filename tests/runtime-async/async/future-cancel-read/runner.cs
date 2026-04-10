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
                await signalWriter.Write();
                await dataWriter.Write(4);
            }

            await WriterAsync();
        }
    }
}
