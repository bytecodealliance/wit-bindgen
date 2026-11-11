import wit.test.resource_alias.test;

import wit.test.resource_alias.e1.exports : Foo1 = Foo, X;
import wit.test.resource_alias.e2.exports : Foo2 = Foo, Bar, Y;

import wit.common;

@witInterface("test:resource-alias/e1") {
    @witExport("x")
    struct XImpl {
        uint val;
        
        @witExport("[constructor]")
        static X constructor(uint v) {
            return X.makeNew((out typeof(this) self) {
                self.val = v;
            });
        }
    }
    
    @witExport("a")
    WitList!X a1(ref scope Foo1 f) {
        // `f.x` consumed by return
    
        immutable X[1] ret = [f.x];
    
        return ret.witList.witClone;
    }
}

@witInterface("test:resource-alias/e2")
@witExport("a")
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
