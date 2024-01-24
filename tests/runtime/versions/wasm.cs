using System.Diagnostics;
using v1 = FooWorld.wit.imports.test.dep.v0_1_0;
using v2 = FooWorld.wit.imports.test.dep.v0_2_0;

namespace FooWorld {

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

namespace FooWorld.wit.exports.test.dep.v0_1_0 {


    public class TestImpl : FooWorld.wit.exports.test.dep.v0_1_0.ITest
    {
        public static float X() {
            return 1.0f;
        }
        
        public static float Y(float a){
            return a + 1.0f;
        }
    }
}

namespace FooWorld.wit.exports.test.dep.v0_2_0
{
    public class TestImpl : FooWorld.wit.exports.test.dep.v0_2_0.ITest
    {
        public static float X() {
            return 2.0f;
        }
        
        public static float Z(float a, float b){
            return a + b + 2.0f;
        }
    }
}