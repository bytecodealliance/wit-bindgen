using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.imports.test.resourceBorrowInRecord;
using System.Text;

public class Program {
    public static void Main() {
        IToTest.Thing thing1 = new IToTest.Thing("Bonjour");
        IToTest.Thing thing2 = new IToTest.Thing("mon cher");

        List<IToTest.Foo> myList = new List<IToTest.Foo>();
        myList.Add(new IToTest.Foo(thing1));
        myList.Add(new IToTest.Foo(thing2));
        List<IToTest.Thing> ret = ToTestInterop.Test(myList);

        Debug.Assert(ret[0].Get() == "Bonjour new test get");
        Debug.Assert(ret[1].Get() == "mon cher new test get");
    }
}
