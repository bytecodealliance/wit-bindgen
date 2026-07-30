import wit.test.list_in_variant.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    const WitString[2] hw = ["hello".witList, "world".witList];
    {
        auto result = listInOption(some(hw[].witList));
        scope(exit) result.witFree;

        assert(result == "hello,world");
    }
    {
        auto result = listInOption(none!(WitList!WitString));
        scope(exit) result.witFree;

        assert(result == "none");
    }

    const WitString[3] fbb_data = ["foo".witList, "bar".witList, "baz".witList];
    auto fbb = PayloadOrEmpty.withData(fbb_data.witList);
    {
        auto result = listInVariant(fbb);
        scope(exit) result.witFree;

        assert(result == "foo,bar,baz");
    }
    {
        auto result = listInVariant(PayloadOrEmpty.empty);
        scope(exit) result.witFree;

        assert(result == "empty");
    }

    const WitString[3] abc = ["a".witList, "b".witList, "c".witList];
    {
        auto result = listInResult(Result!(WitList!WitString, WitString).ok(abc[].witList));
        scope(exit) result.witFree;

        assert(result == "a,b,c");
    }
    {
        auto result = listInResult(Result!(WitList!WitString, WitString).err("oops".witList));
        scope(exit) result.witFree;

        assert(result == "err:oops");
    }

    const WitString[2] hw2 = ["hello".witList, "world".witList];
    auto s1 = listInOptionWithReturn(some(hw2.witList));
    {
        auto result = s1.count;
        scope(exit) result.witFree;

        assert(result == 2);
    }
    {
        auto result = s1.label;
        scope(exit) result.witFree;

        assert(result == "hello,world");
    }
    auto s2 = listInOptionWithReturn(none!(WitList!WitString));
    {
        auto result = s2.count;
        scope(exit) result.witFree;

        assert(result == 0);
    }
    {
        auto result = s2.label;
        scope(exit) result.witFree;

        assert(result == "none");
    }

    const WitString[3] xyz = ["x".witList, "y".witList, "z".witList];
    {
        auto result = topLevelList(xyz.witList);
        scope(exit) result.witFree;

        assert(result == "x,y,z");
    }
}

alias Exports = wit.test.list_in_variant.runner.Exports!(
    run
);
