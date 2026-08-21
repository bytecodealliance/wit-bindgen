import wit.test.resource_alias.test;

import wit.test.resource_alias.e1.exports : Foo1 = Foo, X;
import wit.test.resource_alias.e2.exports : Foo2 = Foo, Bar, Y;

import wit.common;

@witExport("test:resource-alias/e1", "x")
struct XImpl {
    uint val;

    @witExport("test:resource-alias/e1", "[constructor]x")
    static X constructor(uint v) {
        return X.makeNew((out typeof(this) self) {
            self.val = v;
        });
    }
}

@witExport("test:resource-alias/e1", "a")
WitList!X a1(ref scope Foo1 f) {
    // `f.x` consumed by return

    immutable X[1] ret = [f.x];

    return ret.witList.witClone;
}

@witExport("test:resource-alias/e2", "a")
WitList!Y a2(ref scope Foo2 f, ref scope Bar g, Y.Borrow h) {
    // `f.x` consumed by return
    // `f.g` consumed by return
    //scope(exit) h.witDrop;

    immutable X[2] ret = [f.x, g.x];

    return ret.witList.witClone;
}


alias Exports = wit.test.resource_alias.test.Exports!(
    XImpl,
    a1,
    a2
);
