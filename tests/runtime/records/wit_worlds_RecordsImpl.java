package wit.worlds;

import wit.worlds.Records.Tuple0;
import wit.worlds.Records.Tuple1;
import wit.worlds.Records.Tuple2;
import wit.worlds.Records.Tuple4;
import wit.imports.test.records.Test;

public class RecordsImpl {
    public static void testImports() {
        {
            Tuple2<Byte, Short> results = Test.multipleResults();

            expect(results.f0 == (byte) 4);
            expect(results.f1 == (short) 5);
        }

        {
            Tuple2<Integer, Byte> results = Test.swapTuple(new Tuple2<>((byte) 1, 2));

            expect(results.f0 == 2);
            expect(results.f1 == (byte) 1);
        }

        expect(Test.roundtripFlags1(Test.F1.A).value == Test.F1.A.value);
        expect(Test.roundtripFlags1(new Test.F1((byte) 0)).value == (byte) 0);
        expect(Test.roundtripFlags1(Test.F1.B).value == Test.F1.B.value);
        expect(Test.roundtripFlags1(new Test.F1((byte) (Test.F1.A.value | Test.F1.B.value))).value
               == (byte) (Test.F1.A.value | Test.F1.B.value));

        expect(Test.roundtripFlags2(Test.F2.C).value == Test.F2.C.value);
        expect(Test.roundtripFlags2(new Test.F2((byte) 0)).value == (byte) 0);
        expect(Test.roundtripFlags2(Test.F2.D).value == Test.F2.D.value);
        expect(Test.roundtripFlags2(new Test.F2((byte) (Test.F2.C.value | Test.F2.E.value))).value
               == (byte) (Test.F2.C.value | Test.F2.E.value));

        {
            Tuple4<Test.Flag8, Test.Flag16, Test.Flag32, Test.Flag64> results =
                Test.roundtripFlags3(Test.Flag8.B0, Test.Flag16.B1, Test.Flag32.B2, Test.Flag64.B3);

            expect(results.f0.value == Test.Flag8.B0.value);
            expect(results.f1.value == Test.Flag16.B1.value);
            expect(results.f2.value == Test.Flag32.B2.value);
            expect(results.f3.value == Test.Flag64.B3.value);
        }

        {
            Test.R1 result = Test.roundtripRecord1(new Test.R1((byte) 8, Test.F1.A));

            expect(result.a == (byte) 8);
            expect(result.b.value == Test.F1.A.value);
        }

        {
            Test.R1 result = Test.roundtripRecord1
                (new Test.R1((byte) 0, new Test.F1((byte) (Test.F1.A.value | Test.F1.B.value))));

            expect(result.a == (byte) 0);
            expect(result.b.value == (byte) (Test.F1.A.value | Test.F1.B.value));
        }

        Test.tuple0(Tuple0.INSTANCE);

        {
            Tuple1<Byte> result = Test.tuple1(new Tuple1<>((byte) 1));

            expect(result.f0 == 1);
        }
    }

    private static void expect(boolean v) {
        if (!v) {
            throw new AssertionError();
        }
    }
}
