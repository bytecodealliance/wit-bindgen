import wit.test.common.middle;
import wit.common;

import imps = wit.test.common.to_test.imports;

@witExport("test:common/to-test", "wrap")
R1 wrap(in F1 flag) {
    return imps.wrap(flag);
}

@witExport("test:common/to-test", "var-f")
V1 varF() {
    return imps.varF;
}

alias Exports = wit.test.common.middle.Exports!(
    wrap,
    varF
);
