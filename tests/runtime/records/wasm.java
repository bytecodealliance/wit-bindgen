package wit_exports;

import wit_imports.Imports;

public class ExportsImpl {
    public static void testImports() {
        {
            Imports.Tuple2<Byte, Short> results = Imports.multipleResults();

            assert(results.f0 == (byte) 4);
            assert(results.f1 == (short) 5);
        }

        {
            Imports.Tuple2<Integer, Byte> results = Imports.swapTuple(new Imports.Tuple2<>((byte) 1, 2));

            assert(results.f0 == 2);
            assert(results.f1 == (byte) 1);
        }

        assert(Imports.roundtripFlags1(Imports.F1.A).value == Imports.F1.A.value);
        assert(Imports.roundtripFlags1(new Imports.F1((byte) 0)).value == (byte) 0);
        assert(Imports.roundtripFlags1(Imports.F1.B).value == Imports.F1.B.value);
        assert(Imports.roundtripFlags1(new Imports.F1((byte) (Imports.F1.A.value | Imports.F1.B.value))).value
               == (byte) (Imports.F1.A.value | Imports.F1.B.value));

        assert(Imports.roundtripFlags2(Imports.F2.C).value == Imports.F2.C.value);
        assert(Imports.roundtripFlags2(new Imports.F2((byte) 0)).value == (byte) 0);
        assert(Imports.roundtripFlags2(Imports.F2.D).value == Imports.F2.D.value);
        assert(Imports.roundtripFlags2(new Imports.F2((byte) (Imports.F2.C.value | Imports.F2.E.value))).value
               == (byte) (Imports.F2.C.value | Imports.F2.E.value));

        {
            Imports.Tuple4<Imports.Flag8, Imports.Flag16, Imports.Flag32, Imports.Flag64> results =
                Imports.roundtripFlags3(Imports.Flag8.B0, Imports.Flag16.B1, Imports.Flag32.B2, Imports.Flag64.B3);

            assert(results.f0.value == Imports.Flag8.B0.value);
            assert(results.f1.value == Imports.Flag16.B1.value);
            assert(results.f2.value == Imports.Flag32.B2.value);
            assert(results.f3.value == Imports.Flag64.B3.value);
        }

        {
            Imports.R1 result = Imports.roundtripRecord1(new Imports.R1((byte) 8, Imports.F1.A));

            assert(result.a == (byte) 8);
            assert(result.b.value == Imports.F1.A.value);
        }

        {
            Imports.R1 result = Imports.roundtripRecord1
                (new Imports.R1((byte) 0, new Imports.F1((byte) (Imports.F1.A.value | Imports.F1.B.value))));

            assert(result.a == (byte) 0);
            assert(result.b.value == (byte) (Imports.F1.A.value | Imports.F1.B.value));
        }

        Imports.tuple0(Imports.Tuple0.INSTANCE);

        {
            Imports.Tuple1<Byte> result = Imports.tuple1(new Imports.Tuple1<>((byte) 1));

            assert(result.f0 == 1);
        }
    }

    public static Exports.Tuple2<Byte, Short> multipleResults() {
        return new Exports.Tuple2<>((byte) 100, (short) 200);
    }

    public static Exports.Tuple2<Integer, Byte> swapTuple(Exports.Tuple2<Byte, Integer> tuple) {
        return new Exports.Tuple2<>(tuple.f1, tuple.f0);
    }

    public static Exports.F1 roundtripFlags1(Exports.F1 a) {
        return a;
    }

    public static Exports.F2 roundtripFlags2(Exports.F2 a) {
        return a;
    }

    public static Exports.Tuple4<Exports.F8, Exports.F16, Exports.F32, Exports.F64> roundtripFlags3
        (Exports.F8 a, Exports.F16 b, Exports.F32 c, Exports.F64 d)
    {
        return new Exports.Tuple4<>(a, b, c, d);
    }

    public static Exports.R1 roundtripRecord1(Exports.R1 a) {
        return a;
    }

    public static Exports.Tuple0 tuple0(Exports.Tuple0 a) {
        return a;
    }

    public static Exports.Tuple1<Byte> tuple1(Exports.Tuple1<Byte> a) {
        return a;
    }
}
