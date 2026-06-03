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
        auto result = fListInRecord3(const ListInRecord3("list_in_record3 input".witList));
        scope(exit) result.witFree;

        assert(
            result.a
            == "list_in_record3 output"
        );
    }

    {
        auto result = fListInRecord4(const ListInAlias("input4".witList));
        scope(exit) result.witFree;

        assert(
            result.a
            == "result4"
        );
    }

    fListInVariant1(some("foo".witList), Result!(void, WitString).err("bar".witList));

    {
        auto result = fListInVariant2();
        scope(exit) result.witFree;


        assert(
            result
            == some("list_in_variant2".witList)
        );
    }

    {
        auto result = fListInVariant3(some("input3".witList));
        scope(exit) result.witFree;

        assert(
            result
            == some("output3".witList)
        );
    }

    {
        auto errno = errnoResult();
        assert(errno.isErr && errno.unwrapErr == MyErrno.b);
    }
    assert(errnoResult().isOk);

    {
        immutable WitString[1] input = ["typedef2".witList];
        auto result = listTypedefs("typedef1".witList, input[].witList);
        scope(exit) result.witFree;

        assert(result[0] == (cast(ubyte[])"typedef3").witList);
        assert(result[1].length == 1);
        assert(result[1][0] == "typedef4");
    }

    {
        static immutable bool[] input1 = [true, false];
        static immutable Result!()[] input2 = [Result!().ok, Result!().err];
        static immutable MyErrno[] input3 = [MyErrno.success, MyErrno.a];

        auto result = listOfVariants(input1[].witList, input2[].witList, input3[].witList);
        scope(exit) result.witFree;

        static immutable bool[] output1 = [false, true];
        static immutable Result!()[] output2 = [Result!().err, Result!().ok];
        static immutable MyErrno[] output3 = [MyErrno.a, MyErrno.b];
        assert(result[0] == output1);
        assert(result[1] == output2);
        assert(result[2] == output3);
    }
}

alias Exports = wit.test.flavorful.runner.Exports!(
    run
);
