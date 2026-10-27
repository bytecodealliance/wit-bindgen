using System.Runtime.InteropServices;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        static void Expect(string actual, string expected)
        {
            if (actual != expected)
            {
                throw new Exception($"expected \"{expected}\", got \"{actual}\"");
            }
        }

        public static async Task ManyArguments(
            string a, string b, string c, string d, string e, string f, string g, string h,
            string i)
        {
            Expect(a, "a");
            Expect(b, "b");
            Expect(c, "c");
            Expect(d, "d");
            Expect(e, "e");
            Expect(f, "f");
            Expect(g, "g");
            Expect(h, "h");
            Expect(i, "i");
            await Task.CompletedTask;
        }

        public static async Task ManyArgumentsPending(
            FutureReader x, string a, string b, string c, string d, string e, string f,
            string g, string h)
        {
            Expect(a, "a");
            Expect(b, "b");
            Expect(c, "c");
            Expect(d, "d");
            Expect(e, "e");
            Expect(f, "f");
            Expect(g, "g");
            Expect(h, "h");
            // The caller drops this call while it waits here.
            await x.Read();
        }

        public static async Task ManyArgumentsCases(
            uint? a, Result<uint, string> b, string c, string d, string e, string f,
            string g, string h)
        {
            bool someAndErr = a == 42 && b.IsErr && b.AsErr == "error";
            bool noneAndOk = a == null && b.IsOk && b.AsOk == 7;
            if (!someAndErr && !noneAndOk)
            {
                throw new Exception("unexpected option or result");
            }
            Expect(c, "c");
            Expect(d, "d");
            Expect(e, "e");
            Expect(f, "f");
            Expect(g, "g");
            Expect(h, "h");
            await Task.CompletedTask;
        }

        public static async Task OneBigRecord(IIExports.BigRecord r)
        {
            if (r.f0 != 0 || r.f8 != 8 || r.f16 != 16)
            {
                throw new Exception("unexpected record");
            }
            await Task.CompletedTask;
        }

        public static async Task DirectValues(
            string a, string b, string c, string d, string e, string f, string g, uint h)
        {
            Expect(a, "a");
            Expect(b, "b");
            Expect(c, "c");
            Expect(d, "d");
            Expect(e, "e");
            Expect(f, "f");
            Expect(g, "g");
            if (h != 8)
            {
                throw new Exception($"expected 8, got {h}");
            }
            await Task.CompletedTask;
        }

        // The end of the memory that malloc hands out. A block that is never
        // freed makes it grow; once the component has warmed up it stays put.
        [DllImport("*")]
        static extern nint sbrk(nint increment);

        public static uint HeapMark()
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
            return (uint)sbrk(0);
        }

        // The C# async support does not handle a cancelled export yet.
        public static bool SupportsCancel()
        {
            return false;
        }
    }
}
