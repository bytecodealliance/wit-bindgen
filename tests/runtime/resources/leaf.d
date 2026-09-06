import wit.test.resources.leaf;

import wit.common;

@witInterface("imports") {
    @witExport("y")
    struct YImpl {
        int val;

    @witInterface("imports") :

        @witExport("[constructor]y")
        static Y constructor(int a)
        {
            return Y.makeNew((out typeof(this) self) { self.val = a; });
        }

        @witExport("[method]y.get-a")
        int getA()
        {
            return val;
        }

        @witExport("[method]y.set-a")
        void setA(int a)
        {
            val = a;
        }

        @witExport("[static]y.add")
        static Y add(Y y, int a)
        {
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
