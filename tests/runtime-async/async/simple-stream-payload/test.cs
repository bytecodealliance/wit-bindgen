using System.Diagnostics;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static async Task ReadStream(StreamReader<byte> streamReader)
        {
            Console.WriteLine("ReadStream");
            byte[] oneByteBuffer = new byte[1];
            var read = await streamReader.Read(oneByteBuffer);
            Debug.Assert(read == 1);
            Debug.Assert(oneByteBuffer[0] == 0);

            // read two items
            Console.WriteLine("ReadStream 2");
            byte[] twoByteBuffer = new byte[2];
            read = await streamReader.Read(twoByteBuffer);
            Debug.Assert(read == 2);
            Debug.Assert(twoByteBuffer[0] == 1);
            Debug.Assert(twoByteBuffer[1] == 2);

           // read 1/2 items
            Console.WriteLine("ReadStream 1/2");
            read = await streamReader.Read(oneByteBuffer);
            Debug.Assert(read == 1);
            Debug.Assert(oneByteBuffer[0] == 3);

           // read the next buffered item
            Console.WriteLine("ReadStream 2/2");
            read = await streamReader.Read(oneByteBuffer);
            Debug.Assert(read == 1);
            Debug.Assert(oneByteBuffer[0] == 4);

            streamReader.Dispose();

            Console.WriteLine("ReadStream end");
        }
    }
}
