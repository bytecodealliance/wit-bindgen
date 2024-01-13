using System.Diagnostics;
using v1 = wit_foo.wit.imports.test.dep.v0_1_0.Test;
using v2 = wit_foo.wit.imports.test.dep.v0_2_0.Test;

namespace wit_foo {

public class FooWorldImpl : IFooWorld
{
    public static void TestImports()
    {
        Debug.Assert(v1.TestInterop.X() == 1.0f);
        Debug.Assert(v1.TestInterop.Y(1.0f) == 2.0f);

        Debug.Assert(v2.TestInterop.X() == 2.0f);
        Debug.Assert(v2.TestInterop.Z(1.0f, 1.0f) == 4.0f);
    }
}
}

namespace wit_foo.wit.exports.test.dep.v0_1_0.Test {


    public class TestImpl : wit_foo.wit.exports.test.dep.v0_1_0.Test.ITest
    {
        public static float X() {
            return 1.0f;
        }
        
        public static float Y(float a){
            return a + 1.0f;
        }
    }
}

namespace wit_foo.wit.exports.test.dep.v0_2_0.Test {
    public class TestImpl : wit_foo.wit.exports.test.dep.v0_2_0.Test.ITest
    {
        public static float X() {
            return 2.0f;
        }
        
        public static float Z(float a, float b){
            return a + b + 2.0f;
        }
    }
}