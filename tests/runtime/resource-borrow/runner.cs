using RunnerWorld.wit.Imports.test.resourceBorrow;
using System.Diagnostics;

public class RunnerWorldImpl {
    public static void Main() {
        uint ret = IToTestImports.Foo(new IToTestImports.Thing(42));
        Debug.Assert(ret == 42 + 1 + 2);
    }
}
