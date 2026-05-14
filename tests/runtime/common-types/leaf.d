import wit.test.common.leaf;
import wit.common;

@witExport("test:common/to-test", "wrap")
R1 wrap(in F1 flag) {
    switch (flag.bits) with (F1) {
        case a.bits:
            return R1(1, flag);
        case b.bits:
            return R1(2, flag);
        default:
            assert(0);
    }
}

@witExport("test:common/to-test", "var-f")
V1 varF() {
    return V1.b(42);
}

alias Exports = wit.test.common.leaf.Exports!(
    wrap,
    varF
);
