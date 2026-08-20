import wit.test.resource_alias_redux.test;

import wit.test.resource_alias_redux.test.exports.the_test : Thing1 = Thing;
import wit.test.resource_alias_redux.resource_alias1.exports : Foo1 = Foo, Thing2 = Thing;
import wit.test.resource_alias_redux.resource_alias2.exports : Foo2 = Foo;
import wit.common;

extern(C) void* malloc(size_t size);
extern(C) void free(void* ptr);

char[] concat(in char[] a, in char[] b) {
    auto ptr = cast(char*)malloc(a.length + b.length);
    assert((a.length + b.length) == 0 || ptr);

    if (!ptr) return null;

    ptr[0..a.length] = a[];
    ptr[a.length..a.length+b.length] = b[];

    return ptr[0..a.length+b.length];
}

@witExport("test:resource-alias-redux/resource-alias1", "thing")
struct ThingImpl {
    const(char)[] str;

    @witExport("test:resource-alias-redux/resource-alias1", "[constructor]thing")
    static Thing1 constructor(in WitString msg) {
        return Thing1.makeNew((out typeof(this) self) {
            self.str = concat(msg, " GuestThing");
        });
    }

    @witExport("test:resource-alias-redux/resource-alias1", "[method]thing.get")
    WitString get() {
        return concat(str, " GuestThing.get").witList; // no clone; already on C heap
    }
}

@witExport("test:resource-alias-redux/resource-alias1", "a")
WitList!Thing1 a(scope ref Foo1 f) {
    scope(exit) f.witDrop;

    Thing1[1] things = [f.thing];
    f.thing = Thing1.init; // consumed

    return things.witList.witClone;
}

@witExport("test:resource-alias-redux/resource-alias2", "b")
WitList!Thing2 b(scope ref Foo2 f, scope ref Bar g) {
    scope(exit) {
        f.witDrop;
        g.witDrop;
    }

    Thing1[2] things = [f.thing, g.thing];
    f.thing = Thing2.init; // consumed
    g.thing = Thing2.init; // consumed

    return things.witList.witClone;
}

@witExport("the-test", "test")
WitList!Thing1 test(scope ref WitList!Thing1 things) {
    return things.witClone;
}


alias Exports = wit.test.resource_alias_redux.test.Exports!(
    ThingImpl,
    a,
    b,
    test
);
