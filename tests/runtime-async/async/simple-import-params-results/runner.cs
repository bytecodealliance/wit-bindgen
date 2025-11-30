using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.a.b;
using System.Text;

public class Program
{
    public static async Task Main(string[] args)
    {
        var t = IIImports.OneArgument(1);
        Debug.Assert(t.IsCompletedSuccessfully);

        var tOneResult = IIImports.OneResult();
        Debug.Assert(tOneResult.IsCompletedSuccessfully);
        Debug.Assert(tOneResult.Result == 2);

        var tOneArgumentAndResult = IIImports.OneArgumentAndResult(3);
        Debug.Assert(tOneArgumentAndResult.IsCompletedSuccessfully);
        Debug.Assert(tOneArgumentAndResult.Result == 4);

        var tTwoArguments = IIImports.TwoArguments(5, 6);
        Debug.Assert(tTwoArguments.IsCompletedSuccessfully);

        var tTwoArgumentsAndResult = IIImports.TwoArgumentsAndResult(7, 8);
        Debug.Assert(tTwoArgumentsAndResult.IsCompletedSuccessfully);
        Debug.Assert(tTwoArgumentsAndResult.Result == 9);
    }
}
