package wit_records;

import wit_records.RecordsWorld.Tuple0;
import wit_records.RecordsWorld.Tuple1;
import wit_records.RecordsWorld.Tuple2;
import wit_records.RecordsWorld.Tuple4;

public class ExportsImpl {
    public static Tuple2<Byte, Short> multipleResults() {
        return new Tuple2<>((byte) 100, (short) 200);
    }

    public static Tuple2<Integer, Byte> swapTuple(Tuple2<Byte, Integer> tuple) {
        return new Tuple2<>(tuple.f1, tuple.f0);
    }

    public static Exports.F1 roundtripFlags1(Exports.F1 a) {
        return a;
    }

    public static Exports.F2 roundtripFlags2(Exports.F2 a) {
        return a;
    }

    public static Tuple4<Exports.Flag8, Exports.Flag16, Exports.Flag32, Exports.Flag64> roundtripFlags3
        (Exports.Flag8 a, Exports.Flag16 b, Exports.Flag32 c, Exports.Flag64 d)
    {
        return new Tuple4<>(a, b, c, d);
    }

    public static Exports.R1 roundtripRecord1(Exports.R1 a) {
        return a;
    }

    public static Tuple0 tuple0(Tuple0 a) {
        return a;
    }

    public static Tuple1<Byte> tuple1(Tuple1<Byte> a) {
        return a;
    }
}
