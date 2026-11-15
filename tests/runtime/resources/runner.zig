const std = @import("std");
const assert = std.debug.assert;
const wit = @import("wit");
const exports = wit.test_resources_exports;

comptime {
    wit.exports.run(run);
}

fn run() void {
    assert(exports.testImports() == .ok);

    const x = exports.X.init(5);
    assert(x.getA() == 5);
    x.setA(10);
    assert(x.getA() == 10);

    const z1 = exports.Z.init(10);
    assert(z1.getA() == 10);
    const z2 = exports.Z.init(20);
    assert(z2.getA() == 20);

    const xadd = exports.X.add(x, 5);
    assert(xadd.getA() == 15);

    const zadd = exports.add(z1, z2);
    assert(zadd.getA() == 30);

    const dropped_zs_start = exports.Z.numDropped();

    z1.deinit();
    z2.deinit();

    exports.consume(xadd);

    const dropped_zs_end = exports.Z.numDropped();
    if (dropped_zs_start != 0) {
        assert(dropped_zs_end == dropped_zs_start + 2);
    }
}
