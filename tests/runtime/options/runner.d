import wit.test.options.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    optionNoneParam(none!WitString);
    optionSomeParam("foo".witList.some);
    assert(optionNoneResult().isNone);
    {
        auto result = optionSomeResult();
        scope(exit) result.witFree;

        assert(result == "foo".witList.some);
    }
    {
        auto result = optionRoundtrip("foo".witList.some);
        scope(exit) result.witFree;

        assert(result == "foo".witList.some);
    }
    {
        auto result = doubleOptionRoundtrip(uint(42).some.some);
        scope(exit) result.witFree;

        assert(result == uint(42).some.some);
    }
    {
        auto result = doubleOptionRoundtrip(none!uint.some);
        scope(exit) result.witFree;

        assert(result == none!uint.some);
    }
    {
        auto result = doubleOptionRoundtrip(none!(Option!uint));
        scope(exit) result.witFree;

        assert(result == none!(Option!uint));
    }
}

alias Exports = wit.test.options.runner.Exports!(
    run
);
