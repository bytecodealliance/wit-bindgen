import wit.test.resource_floats.intermediate;

import wit.test.resource_floats.intermediate.exports.exports : EFloat = Float;

import wit.test.resource_floats.intermediate.imports.imports : IFloat1 = Float;
import wit.test.resource_floats.test.imports : IFloat2 = Float;

import wit.common;

@witInterface("exports")
@witExport("float")
struct FloatImpl {
    IFloat1 val;

    @witExport("[constructor]")
    static EFloat constructor(double v) {
        return EFloat.makeNew((out typeof(this) self) {
            self.val = IFloat1.makeNew(v + 1);
        });
    }

    @witExport("get")
    double get() {
        return val.get + 3;
    }

    @witExport("add")
    static EFloat add(EFloat a, double b) {
        scope(exit) a.witDrop;

        auto added = IFloat1.add(a.rep!FloatImpl.val, b);
        scope(exit) added.witDrop;

        return FloatImpl.constructor(
            added.get + 5
        );
    }
}

@witInterface("$root")
@witExport("add")
static IFloat2 add(IFloat2.Borrow a, IFloat2.Borrow b) {
    scope(exit) {
        a.witDrop;
        b.witDrop;
    }

    return IFloat2.makeNew(
        a.get + b.get + 5
    );
}

alias Exports = wit.test.resource_floats.intermediate.Exports!(
    FloatImpl,
    add
);
