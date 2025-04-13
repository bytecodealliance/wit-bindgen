using System.Diagnostics;

namespace TestWorld.wit.exports.test.dep.v0_1_0 {
    public class TestImpl : TestWorld.wit.exports.test.dep.v0_1_0.ITest
    {
        public static float X() {
            return 1.0f;
        }
        
        public static float Y(float a){
            return a + 1.0f;
        }
    }
}

namespace TestWorld.wit.exports.test.dep.v0_2_0
{
    public class TestImpl : TestWorld.wit.exports.test.dep.v0_2_0.ITest
    {
        public static float X() {
            return 2.0f;
        }
        
        public static float Z(float a, float b){
            return a + b + 2.0f;
        }
    }
}
