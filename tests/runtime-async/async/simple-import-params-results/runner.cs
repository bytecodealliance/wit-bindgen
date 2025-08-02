using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.a.b;
using System.Text;

public class Program
{
    public static async Task Main(string[] args)
    {
        var t = II.OneArgument(1);
        Debug.Assert(t.IsCompletedSuccessfully);

        var tOneResult = II.OneResult();
        Debug.Assert(tOneResult.IsCompletedSuccessfully);
        Debug.Assert(tOneResult.Result == 2);

        var tOneArgumentAndResult = II.OneArgumentAndResult(3);
        Debug.Assert(tOneArgumentAndResult.IsCompletedSuccessfully);
        Debug.Assert(tOneArgumentAndResult.Result == 4);

        var tTwoArguments = II.TwoArguments(5, 6);
        Debug.Assert(tTwoArguments.IsCompletedSuccessfully);

        var tTwoArgumentsAndResult = II.TwoArgumentsAndResult(7, 8);
        Debug.Assert(tTwoArgumentsAndResult.IsCompletedSuccessfully);
        Debug.Assert(tTwoArgumentsAndResult.Result == 9);
    }
}
