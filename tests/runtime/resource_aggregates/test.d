import wit.test.resource_aggregates.test;

import wit.common;
import std.algorithm.iteration : sum, map;

@witExport("test:resource-aggregates/to-test", "thing")
struct ThingImpl {
    uint val;

    @witExport("test:resource-aggregates/to-test", "[constructor]thing")
    static Thing constructor(uint v) {
        return Thing.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }
}

@witExport("test:resource-aggregates/to-test", "foo")
uint foo(
    scope ref R1 r1, scope ref R2 r2, scope ref R3 r3,
    scope ref T1 t1, scope ref T2 t2,
    scope ref V1 v1, scope ref V2 v2,
    scope ref L1 l1, scope ref L2 l2,
    scope ref Option!Thing o1, scope ref Option!(Thing.Borrow) o2,
    scope ref Result!(Thing, void) result1, scope ref Result!(Thing.Borrow, void) result2
) {
    scope(exit) {
        r1.witDrop;
        r2.witDrop;
        r3.witDrop;
        t1.witDrop;
        t2.witDrop;
        v1.witDrop;
        v2.witDrop;
        l1.witDrop;
        l2.witDrop;
        o1.witDrop;
        o2.witDrop;
        result1.witDrop;
        result2.witDrop;
    }

    return (
          r1.thing.rep!ThingImpl.val
        + r2.thing.rep!ThingImpl.val
        + r3.thing1.rep!ThingImpl.val
        + r3.thing2.rep!ThingImpl.val
        + t1[0].rep!ThingImpl.val
        + t1[1].thing.rep!ThingImpl.val
        + t2[0].rep!ThingImpl.val
        + v1.getThing.rep!ThingImpl.val
        + v2.getThing.rep!ThingImpl.val
        + l1[].map!((a) => a.rep!ThingImpl.val).sum
        + l2[].map!((a) => a.rep!ThingImpl.val).sum
        + o1.unwrap.rep!ThingImpl.val
        + o2.unwrap.rep!ThingImpl.val
        + result1.unwrap.rep!ThingImpl.val
        + result2.unwrap.rep!ThingImpl.val
        + 3
    );
}

alias Exports = wit.test.resource_aggregates.test.Exports!(
    ThingImpl,
    foo
);
