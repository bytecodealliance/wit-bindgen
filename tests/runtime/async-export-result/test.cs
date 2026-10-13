using System.Threading.Tasks;

namespace TestWorld.wit.Exports.my.test
{
    public class IExportsImpl : IIExports
    {
        public static async Task UnitOk()
        {
            await Task.CompletedTask;
        }

        public static async Task UnitErr()
        {
            await Task.CompletedTask;
            throw new WitException("unit", 0);
        }

        public static async Task<uint> Halve(uint x)
        {
            await Task.CompletedTask;
            if (x % 2 != 0)
            {
                throw new WitException("odd", 0);
            }
            return x / 2;
        }
    }
}
