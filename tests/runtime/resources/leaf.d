import wit.test.resources.leaf;

import wit.common;

@witExport("imports", "y")
struct YImpl {
    int val;

    @witExport("imports", "[constructor]y")
    static Y constructor(int a) {
        return Y.makeNew((out typeof(this) self) {
            self.val = a;
        });
    }

    @witExport("imports", "[method]y.get-a")
    int getA() {
        return val;
    }

    @witExport("imports", "[method]y.set-a")
    void setA(int a) {
        val = a;
    }

    @witExport("imports", "[static]y.add")
    static Y add(Y y, int a) {
        scope(exit) y.witDrop;

        return Y.makeNew((out typeof(this) self) {
            self.val = y.rep!YImpl.getA + a;
        });
    }
}

alias Exports = wit.test.resources.leaf.Exports!(
    YImpl
);
