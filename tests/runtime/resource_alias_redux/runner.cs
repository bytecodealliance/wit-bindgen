using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.resourceAliasRedux;
using RunnerWorld.wit.Imports;
using System.Text;

namespace RunnerWorld;

public class RunnerWorldExportsImpl : IRunnerWorldExports
{
    public static void Run()
    {
        IResourceAlias1Imports.Thing thing1 = new IResourceAlias1Imports.Thing("Ni Hao");
        List<IResourceAlias1Imports.Thing> myList = new List<IResourceAlias1Imports.Thing>();
        myList.Add(thing1);
        List<IResourceAlias1Imports.Thing> ret = ITheTestImports.Test(myList);
        Debug.Assert(ret[0].Get() == "Ni Hao GuestThing GuestThing.get");

        ret = IResourceAlias1Imports.A(
            new IResourceAlias1Imports.Foo(new IResourceAlias1Imports.Thing("Ciao")));
        Debug.Assert(ret[0].Get() == "Ciao GuestThing GuestThing.get");

        ret = IResourceAlias2Imports.B(
            new IResourceAlias2Imports.Foo(new IResourceAlias1Imports.Thing("Ciao")),
            new IResourceAlias1Imports.Foo(new IResourceAlias1Imports.Thing("Aloha"))
        );
        Debug.Assert(ret[0].Get() == "Ciao GuestThing GuestThing.get");
        Debug.Assert(ret[1].Get() == "Aloha GuestThing GuestThing.get");
    }
}
