import wit.test.common.middle;
import wit.common;

import imps = wit.test.common.to_test.imports;

@witInterface("test:common/to-test") {
    @witExport
    R1 wrap(F1 flag) {
        return imps.wrap(flag);
    }
    
    @witExport
    V1 varF() {
        return imps.varF;
    }
}

alias Exports = wit.test.common.middle.Exports!(
    wrap,
    varF
);
