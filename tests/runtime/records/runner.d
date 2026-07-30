import wit.test.records.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    assert(multipleResults() == tuple(ubyte(4), ushort(5)));

    assert(swapTuple(tuple(ubyte(1), uint(2))) == tuple(uint(2), ubyte(1)));
    assert(roundtripFlags1(F1.a) == F1.a);
    assert(roundtripFlags1(F1()) == F1());
    assert(roundtripFlags1(F1.b) == F1.b);
    assert(roundtripFlags1(F1.a | F1.b) == (F1.a | F1.b));

    assert(roundtripFlags2(F2.c) == F2.c);
    assert(roundtripFlags2(F2()) == F2());
    assert(roundtripFlags2(F2.d) == F2.d);
    assert(roundtripFlags2(F2.c | F2.e) == (F2.c | F2.e));

    assert(
        roundtripFlags3(Flag8.b0, Flag16.b1, Flag32.b2) ==
        tuple(Flag8.b0, Flag16.b1, Flag32.b2)
    );

    {
        auto r = roundtripRecord1(R1(
            a: 8,
            b: F1()
        ));
        assert(r.a == 8);
        assert(r.b == F1());
    }

    {
        auto r = roundtripRecord1(R1(
            a: 0,
            b: F1.a | F1.b
        ));
        assert(r.a == 0);
        assert(r.b == (F1.a | F1.b));
    }

    assert(tuple1(tuple(ubyte(1))) == tuple(1));
}

alias Exports = wit.test.records.runner.Exports!(
    run
);
