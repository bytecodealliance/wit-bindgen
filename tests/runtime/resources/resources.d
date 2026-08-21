import wit.test.resources.resources;

import wit.common;

@witExport("exports", "x")
struct XImpl {
    int val;

    @witExport("exports", "[constructor]x")
    static X constructor(int a) {
        return X.makeNew((out typeof(this) self) {
            self.val = a;
        });
    }

    @witExport("exports", "[method]x.get-a")
    int getA() {
        return val;
    }

    @witExport("exports", "[method]x.set-a")
    void setA(int a) {
        val = a;
    }

    @witExport("exports", "[static]x.add")
    static X add(X x, int a) {
        scope(exit) x.witDrop;

        return X.makeNew((out typeof(this) self) {
            self.val = x.rep!XImpl.getA + a;
        });
    }
}

@witExport("exports", "z")
struct ZImpl {
    static uint numDropped = 0;

    int val;

    @witExport("exports", "[constructor]z")
    static Z constructor(int a) {
        return Z.makeNew((out typeof(this) self) {
            self.val = a;
        });
    }

    @witExport("exports", "[method]z.get-a")
    int getA() {
        return val;
    }

    @witExport("exports", "[static]z.num-dropped")
    static uint getNumDropped() {
        return numDropped + 1;
    }

    ~this() {
        numDropped += 1;
    }
}

@witExport("exports", "kebab-case")
struct KebabCaseImpl {
    uint val;

    @witExport("exports", "[constructor]kebab-case")
    static KebabCase constructor(uint a) {
        return KebabCase.makeNew((out typeof(this) self) {
            self.val = a;
        });
    }

    @witExport("exports", "[method]kebab-case.get-a")
    uint getA() {
        return val;
    }

    @witExport("exports", "[static]kebab-case.take-owned")
    static uint takeOwned(KebabCase k) {
        scope(exit) k.witDrop;

        return k.rep!KebabCaseImpl.getA;
    }
}


@witExport("exports", "add")
Z add(Z.Borrow a, Z.Borrow b) {
    scope(exit) {
        a.witDrop;
        b.witDrop;
    }

    return ZImpl.constructor(a.rep!ZImpl.val + b.rep!ZImpl.val);
}


@witExport("exports", "consume")
void consume(X x) {
    x.witDrop;
}


@witExport("exports", "test-imports")
Result!(void, WitString) testImports() {
    {
        auto y = Y.makeNew(10);
        scope(exit) y.witDrop;
        assert(y.getA == 10);
        y.setA(20);
        assert(y.getA == 20);

        auto y2 = Y.add(y, 20);
        scope(exit) y2.witDrop;
        y = Y.init; // consumed
        assert(y2.getA == 40);
    }

    {
        auto y1 = Y.makeNew(1);
        scope(exit) y1.witDrop;
        auto y2 = Y.makeNew(2);
        scope(exit) y2.witDrop;

        assert(y1.getA == 1);
        assert(y2.getA == 2);

        y1.setA(10);
        y2.setA(20);
        assert(y1.getA == 10);
        assert(y2.getA == 20);

        auto y3 = Y.add(y1, 20);
        scope(exit) y3.witDrop;
        y1 = Y.init; // consumed
        assert(y3.getA == 30);

        auto y4 = Y.add(y2, 30);
        scope(exit) y4.witDrop;
        y2 = Y.init; // consumed
        assert(y4.getA == 50);
    }


    return ok!WitString;
}



alias Exports = wit.test.resources.resources.Exports!(
    XImpl,
    ZImpl,
    KebabCaseImpl,

    add,
    consume,
    testImports
);
