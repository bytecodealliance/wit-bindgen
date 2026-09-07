import wit.test.resource_floats.leaf;

import wit.test.resource_floats.leaf.exports.imports : Float;

import wit.test.resource_floats.test.exports : Float2 = Float;
import wit.common;

@witInterface("imports")
@witExport("float")
struct FloatImpl {
    double val;

    @witExport("[constructor]")
    static Float constructor(double v) {
        return Float.makeNew((out typeof(this) self) {
            self.val = v + 2;
        });
    }

    @witExport("get")
    double get() {
        return val + 4;
    }

    @witExport("add")
    static Float add(Float a, double b) {
        scope(exit) a.witDrop;

        return FloatImpl.constructor(
            a.rep!FloatImpl.val + b + 6
        );
    }
}

@witInterface("test:resource-floats/test")
@witExport("float")
struct Float2Impl {
    double val;

    @witExport("[constructor]")
    static Float2 constructor(double v) {
        return Float2.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }

    @witExport("get")
    double get() {
        return val + 3;
    }
}

alias Exports = wit.test.resource_floats.leaf.Exports!(
    FloatImpl,
    Float2Impl
);
