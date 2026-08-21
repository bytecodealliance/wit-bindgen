import wit.test.resource_aggregates.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    auto r2Thing = Thing.makeNew(1);
    scope(exit) r2Thing.witDrop;

    auto r3Thing1 = Thing.makeNew(2);
    scope(exit) r3Thing1.witDrop;

    auto t2Thing = Thing.makeNew(6);
    scope(exit) t2Thing.witDrop;

    auto v2Thing = Thing.makeNew(8);
    scope(exit) v2Thing.witDrop;

    auto l2Thing1 = Thing.makeNew(11);
    scope(exit) l2Thing1.witDrop;

    auto l2Thing2 = Thing.makeNew(12);
    scope(exit) l2Thing2.witDrop;

    auto o2Thing = Thing.makeNew(14);
    scope(exit) o2Thing.witDrop;

    auto result2Thing = Thing.makeNew(16);
    scope(exit) result2Thing.witDrop;

    immutable Thing[2] l1Elems = [
        Thing.makeNew(9),
        Thing.makeNew(10),
    ];

    immutable Thing.Borrow[2] l2Elems = [
        l2Thing1,
        l2Thing2,
    ];

    assert(foo(
        R1(thing: Thing.makeNew(0)),
        R2(thing: r2Thing),
        R3(
            thing1: r3Thing1,
            thing2: Thing.makeNew(3)
        ),
        tuple(
            Thing.makeNew(4),
            R1(thing: Thing.makeNew(5))
        ),
        tuple(t2Thing.borrow),
        V1.thing(Thing.makeNew(7)),
        V2.thing(v2Thing),
        l1Elems.witList,
        l2Elems.witList,
        Thing.makeNew(13).some,
        o2Thing.borrow.some,
        Thing.makeNew(15).ok!void,
        result2Thing.borrow.ok!void
    ) == 156);
}

alias Exports = wit.test.resource_aggregates.runner.Exports!(
    run
);
