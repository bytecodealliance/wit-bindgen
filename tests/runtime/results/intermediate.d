import wit.test.results.intermediate;
import imports = wit.test.results.test.imports;

import wit.common;

@witInterface("test:results/test") {
    @witExport
    Result!(float, WitString) stringError(float a) {
        return imports.stringError(a);
    }
    
    @witExport
    Result!(float, E) enumError(float a) {
        return imports.enumError(a);
    }
    
    @witExport
    Result!(float, E2) recordError(float a) {
        return imports.recordError(a);
    }
    
    @witExport
    Result!(float, E3) variantError(float a) {
        return imports.variantError(a);
    }
    
    @witExport
    Result!(uint, void) emptyError(uint a) {
        return imports.emptyError(a);
    }
    
    @witExport
    Result!(Result!(void, WitString), WitString) doubleError(uint a) {
        return imports.doubleError(a);
    }
}

alias Exports = wit.test.results.intermediate.Exports!(
    stringError,
    enumError,
    recordError,
    variantError,
    emptyError,
    doubleError
);
