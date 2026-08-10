import wit.test.variants.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    assert(roundtripOption(1.0f.some) == ubyte(1).some);
    assert(roundtripOption(none!float) == none!ubyte);
    assert(roundtripOption(2.0f.some) == ubyte(2).some);
    assert(roundtripOption(4.0f.some) == ubyte(4).some);
    assert(roundtripOption(5.3f.some) == ubyte(5).some);

    assert(roundtripEnum(E1.a) == E1.a);
    assert(roundtripEnum(E1.b) == E1.b);

    assert(invertBool(true) == false);
    assert(invertBool(false) == true);

    {
        auto result = variantCasts(tuple(
            C1.a(1),
            C2.a(2),
            C3.a(3),
            C4.a(4),
            C5.a(5),
            C6.a(6.0),
        ));

        assert(result[0] == C1.a(1));
        assert(result[1] == C2.a(2));
        assert(result[2] == C3.a(3));
        assert(result[3] == C4.a(4));
        assert(result[4] == C5.a(5));
        assert(result[5] == C6.a(6.0));
    }

    {
        auto result = variantCasts(tuple(
            C1.b(1),
            C2.b(2.0),
            C3.b(3.0),
            C4.b(4.0),
            C5.b(5.0),
            C6.b(6.0),
        ));

        assert(result[0] == C1.b(1));
        assert(result[1] == C2.b(2.0));
        assert(result[2] == C3.b(3.0));
        assert(result[3] == C4.b(4.0));
        assert(result[4] == C5.b(5.0));
        assert(result[5] == C6.b(6.0));
    }

    {
        auto result = variantZeros(tuple(
            Z1.a(1),
            Z2.a(2),
            Z3.a(3.0),
            Z4.a(4.0),
        ));

        assert(result[0] == Z1.a(1));
        assert(result[1] == Z2.a(2));
        assert(result[2] == Z3.a(3.0));
        assert(result[3] == Z4.a(4.0));
    }

    {
        auto result = variantZeros(tuple(
            Z1.b,
            Z2.b,
            Z3.b,
            Z4.b,
        ));

        assert(result[0] == Z1.b);
        assert(result[1] == Z2.b);
        assert(result[2] == Z3.b);
        assert(result[3] == Z4.b);
    }

    variantTypedefs(none!uint, false, err!uint);

    assert(
        variantEnums(true, ok!void, MyErrno.success)
        == tuple(true, ok!void, MyErrno.success)
    );
}

alias Exports = wit.test.variants.runner.Exports!(
    run
);
