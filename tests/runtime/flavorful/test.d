import wit.test.flavorful.test;
import wit.common;

@witExport("test:flavorful/to-test", "f-list-in-record1")
void fListInRecord1(in ListInRecord1 a) {
    assert(a.a == "list_in_record1");
}

@witExport("test:flavorful/to-test", "f-list-in-record2")
ListInRecord2 fListInRecord2() {
    return (const ListInRecord2("list_in_record2".witList)).witClone;
}

@witExport("test:flavorful/to-test", "f-list-in-record3")
ListInRecord3 fListInRecord3(in ListInRecord3 a) {
    assert(a.a == "list_in_record3 input");
    return (const ListInRecord3("list_in_record3 output".witList)).witClone;
}

@witExport("test:flavorful/to-test", "f-list-in-record4")
ListInAlias fListInRecord4(in ListInAlias a) {
    assert(a.a == "input4");
    return (const ListInAlias("result4".witList)).witClone;
}

@witExport("test:flavorful/to-test", "f-list-in-variant1")
void fListInVariant1(in ListInVariant1V1 a, in ListInVariant1V2 b) {
    assert(a.unwrap() == "foo");
    assert(b.unwrapErr() == "bar");
}

@witExport("test:flavorful/to-test", "f-list-in-variant2")
Option!WitString fListInVariant2() {
    return some("list_in_variant2".witList).witClone;
}

@witExport("test:flavorful/to-test", "f-list-in-variant3")
Option!WitString fListInVariant3(in ListInVariant3 a) {
    assert(a.unwrap() == "input3");
    return some("output3".witList).witClone;
}

@witExport("test:flavorful/to-test", "errno-result")
Result!(void, MyErrno) errnoResult() {
    static bool first = true;

    if (first) {
        first = false;
        return Result!(void, MyErrno).err(MyErrno.b);
    } else {
        return Result!(void, MyErrno).ok();
    }
}


@witExport("test:flavorful/to-test", "list-typedefs")
Tuple!(ListTypedef2, ListTypedef3) listTypedefs(in ListTypedef a, in ListTypedef3 b) {
    assert(a == "typedef1");
    assert(b.length == 1);
    assert(b[0] == "typedef2");

    WitString[1] strings = [
        cast(WitString)"typedef4".witList
    ];

    return (const Tuple!(ListTypedef2, ListTypedef3)((cast(immutable ubyte[])"typedef3").witList, strings[].witList)).witClone;
}



@witExport("test:flavorful/to-test", "list-of-variants")
Tuple!(WitList!bool, WitList!(Result!()), WitList!MyErrno) listOfVariants(in WitList!bool bools, in WitList!(Result!()) results, in WitList!MyErrno enums) {
    static immutable bool[] boolsCmp = [true, false];
    assert(bools == boolsCmp[]);

    static immutable Result!()[] resultsCmp = [Result!().ok, Result!().err];
    assert(results == resultsCmp[]);

    static immutable MyErrno[] enumsCmp = [MyErrno.success, MyErrno.a];
    assert(enums == enumsCmp[]);

    static immutable bool[] boolsOut = [false, true];
    static immutable Result!(void)[] resultsOut = [Result!().err, Result!().ok];
    static immutable MyErrno[] enumsOut = [MyErrno.a, MyErrno.b];
    return (const Tuple!(WitList!bool, WitList!(Result!()), WitList!MyErrno)(
        boolsOut.witList,
        resultsOut.witList,
        enumsOut.witList
    )).witClone;
}

alias Exports = wit.test.flavorful.test.Exports!(
    fListInRecord1,
    fListInRecord2,
    fListInRecord3,
    fListInRecord4,
    fListInVariant1,
    fListInVariant2,
    fListInVariant3,
    errnoResult,
    listTypedefs,
    listOfVariants
);
