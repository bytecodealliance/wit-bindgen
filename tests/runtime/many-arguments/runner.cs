using System;
using System.Runtime.InteropServices;
using System.Diagnostics;
using RunnerWorld.wit.Imports.test.manyArguments;

public class Program
{
    public static void Main(string[] args)
    {
        IToTestImports.ManyArguments(
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16
        );
    }
}

