import wit.test.results.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    {
        auto result = stringError(0.0);
        scope(exit) result.witFree;

        assert(result == "zero".witList.err!float);
    }

    {
        auto result = stringError(1.0);
        scope(exit) result.witFree;

        assert(result == 1.0f.ok!WitString);
    }

    assert(enumError(0.0) == E.a.err!float);
    assert(enumError(1.0) == 1.0f.ok!E);

    assert(recordError(0.0) == E2(
        line: 420,
        column: 0
    ).err!float);
    assert(recordError(1.0) == E2(
        line: 77,
        column: 2
    ).err!float);
    assert(recordError(2.0).isOk);

    assert(variantError(0.0) == E3.e2(E2(
        line: 420,
        column: 0
    )).err!float);
    assert(variantError(1.0) == E3.e1(E.b).err!float);
    assert(variantError(2.0) == E3.e1(E.c).err!float);

    assert(emptyError(0) == err!uint);
    assert(emptyError(1) == 42u.ok!void);
    assert(emptyError(2) == 2u.ok!void);

    {
        auto result = doubleError(0);
        scope(exit) result.witFree;

        assert(result == ok!WitString.ok!WitString);
    }

    {
        auto result = doubleError(1);
        scope(exit) result.witFree;

        assert(result == "one".witList.err!void.ok!WitString);
    }

    {
        auto result = doubleError(2);
        scope(exit) result.witFree;

        assert(result == "two".witList.err!(Result!(void, WitString)));
    }
}

alias Exports = wit.test.results.runner.Exports!(
    run
);
