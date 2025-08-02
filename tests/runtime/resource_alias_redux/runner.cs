using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.resourceAliasRedux;
using RunnerWorld.wit.imports;
using System.Text;

public class Program {
    public static void Main() {
        IResourceAlias1.Thing thing1 = new IResourceAlias1.Thing("Ni Hao");
        List<IResourceAlias1.Thing> myList = new List<IResourceAlias1.Thing>();
        myList.Add(thing1);
        List<IResourceAlias1.Thing> ret = TheTestInterop.Test(myList);
        Debug.Assert(ret[0].Get() == "Ni Hao GuestThing GuestThing.get");

        ret = ResourceAlias1Interop.A(
            new IResourceAlias1.Foo(new IResourceAlias1.Thing("Ciao")));
        Debug.Assert(ret[0].Get() == "Ciao GuestThing GuestThing.get");

        ret = ResourceAlias2Interop.B(
            new IResourceAlias2.Foo(new IResourceAlias1.Thing("Ciao")),
            new IResourceAlias1.Foo(new IResourceAlias1.Thing("Aloha"))
        );
        Debug.Assert(ret[0].Get() == "Ciao GuestThing GuestThing.get");
        Debug.Assert(ret[1].Get() == "Aloha GuestThing GuestThing.get");
    }
}
