using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using v1 = RunnerWorld.wit.imports.test.dep.v0_1_0;
using v2 = RunnerWorld.wit.imports.test.dep.v0_2_0;
using System.Text;

public class Program 
{
    public static void Main(string[] args){
        Debug.Assert(v1.TestInterop.X() == 1.0f);
        Debug.Assert(v1.TestInterop.Y(1.0f) == 2.0f);

        Debug.Assert(v2.TestInterop.X() == 2.0f);
        Debug.Assert(v2.TestInterop.Z(1.0f, 1.0f) == 4.0f);
    }
}
