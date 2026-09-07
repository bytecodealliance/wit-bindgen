import wit.test.resources.leaf;

import wit.common;

@witInterface("imports") {
    @witExport("y")
    struct YImpl {
        int val;

        @witExport("[constructor]")
        static Y constructor(int a) {
            return Y.makeNew((out typeof(this) self) { self.val = a; });
        }

        @witExport("get-a")
        int getA() {
            return val;
        }

        @witExport("set-a")
        void setA(int a) {
            val = a;
        }

        @witExport("add")
        static Y add(Y y, int a) {
            scope (exit)
                y.witDrop;

            return Y.makeNew((out typeof(this) self) {
                self.val = y.rep!YImpl.getA + a;
            });
        }
    }
}

alias Exports = wit.test.resources.leaf.Exports!(
    YImpl
);
