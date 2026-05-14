import wit.test.flavorful.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    fListInRecord1(ListInRecord1(a: cast(WitString)"list_in_record1".witList));

    {
        auto result = fListInRecord2();
        scope(exit) result.witFree;

        assert(result.a == "list_in_record2");
    }

    {
        auto result = fListInRecord3(ListInRecord3(a: cast(WitString)"list_in_record3 input".witList));
        scope(exit) result.witFree;

        assert(
            result.a
            == "list_in_record3 output"
        );
    }

    {
        auto result = fListInRecord4(ListInAlias(a: cast(WitString)"input4".witList));
        scope(exit) result.witFree;

        assert(
            result.a
            == "result4"
        );
    }

    fListInVariant1(some(cast(WitString)"foo".witList), Result!(void, WitString).err(cast(WitString)"bar".witList));

    {
        auto result = fListInVariant2();
        scope(exit) result.witFree;


        assert(
            result
            == some(cast(WitString)"list_in_variant2".witList)
        );
    }

    {
        auto result = fListInVariant3(some(cast(WitString)"input3".witList));
        scope(exit) result.witFree;

        assert(
            result
            == some(cast(WitString)"output3".witList)
        );
    }

    {
        auto errno = errnoResult();
        assert(errno.isErr && errno.unwrapErr == MyErrno.b);
    }
    assert(errnoResult().isOk);

    {
        WitString[1] input = [cast(WitString)"typedef2".witList];
        auto result = listTypedefs(cast(WitString)"typedef1".witList, input[].witList);
        scope(exit) result.witFree;

        assert(result[0] == (cast(ubyte[])"typedef3").witList);
        assert(result[1].length == 1);
        assert(result[1][0] == "typedef4");
    }

    {
        bool[2] input1 = [true, false];
        Result!(void, void)[2] input2 = [Result!(void, void).ok(), Result!(void, void).err()];
        MyErrno[2] input3 = [MyErrno.success, MyErrno.a];

        auto result = listOfVariants(input1[].witList, input2[].witList, input3[].witList);
        scope(exit) result.witFree;
    }
}

alias Exports = wit.test.flavorful.runner.Exports!(
    run
);
