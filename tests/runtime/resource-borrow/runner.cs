using RunnerWorld.wit.imports.test.resourceBorrow;
using System.Diagnostics;

public class RunnerWorldImpl {
    public static void Main() {
        uint ret = ToTestInterop.Foo(new IToTest.Thing(42));
        Debug.Assert(ret == 42 + 1 + 2);
    }
}
