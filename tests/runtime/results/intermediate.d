import wit.test.results.intermediate;
import imports = wit.test.results.test.imports;

import wit.common;

@witExport("test:results/test", "string-error")
Result!(float, WitString) stringError(float a) {
    return imports.stringError(a);
}

@witExport("test:results/test", "enum-error")
Result!(float, E) enumError(float a) {
    return imports.enumError(a);
}

@witExport("test:results/test", "record-error")
Result!(float, E2) recordError(float a) {
    return imports.recordError(a);
}

@witExport("test:results/test", "variant-error")
Result!(float, E3) variantError(float a) {
    return imports.variantError(a);
}

@witExport("test:results/test", "empty-error")
Result!(uint, void) emptyError(uint a) {
    return imports.emptyError(a);
}

@witExport("test:results/test", "double-error")
Result!(Result!(void, WitString), WitString) doubleError(uint a) {
    return imports.doubleError(a);
}

alias Exports = wit.test.results.intermediate.Exports!(
    stringError,
    enumError,
    recordError,
    variantError,
    emptyError,
    doubleError
);
