import wit.test.resource_floats.leaf;

import wit.test.resource_floats.leaf.exports.imports : Float;

import wit.test.resource_floats.test.exports : Float2 = Float;
import wit.common;

@witExport("imports", "float")
struct FloatImpl {
    double val;

    @witExport("imports", "[constructor]float")
    static Float constructor(double v) {
        return Float.makeNew((out typeof(this) self) {
            self.val = v + 2;
        });
    }

    @witExport("imports", "[method]float.get")
    double get() {
        return val + 4;
    }

    @witExport("imports", "[static]float.add")
    static Float add(Float a, double b) {
        scope(exit) a.witDrop;

        return FloatImpl.constructor(
            a.rep!FloatImpl.val + b + 6
        );
    }
}

@witExport("test:resource-floats/test", "float")
struct Float2Impl {
    double val;

    @witExport("test:resource-floats/test", "[constructor]float")
    static Float2 constructor(double v) {
        return Float2.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }

    @witExport("test:resource-floats/test", "[method]float.get")
    double get() {
        return val + 3;
    }
}

alias Exports = wit.test.resource_floats.leaf.Exports!(
    FloatImpl,
    Float2Impl
);
