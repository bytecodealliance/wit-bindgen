using System.Diagnostics;
using RunnerWorld.wit.Imports.test.resourceBorrowInRecord;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        IToTestImports.Thing thing1 = new IToTestImports.Thing("Bonjour");
        IToTestImports.Thing thing2 = new IToTestImports.Thing("mon cher");

        List<IToTestImports.Foo> myList = new List<IToTestImports.Foo>();
        myList.Add(new IToTestImports.Foo(thing1));
        myList.Add(new IToTestImports.Foo(thing2));
        List<IToTestImports.Thing> ret = IToTestImports.Test(myList);

        Debug.Assert(ret[0].Get() == "Bonjour new test get");
        Debug.Assert(ret[1].Get() == "mon cher new test get");
    }
}
