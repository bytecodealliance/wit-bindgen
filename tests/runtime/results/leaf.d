import wit.test.results.leaf;

import wit.common;

@witInterface("test:results/test") {
    @witExport
    Result!(float, WitString) stringError(float a) {
        if (a == 0.0) {
            return "zero".witList.witClone.err!float;
        }
    
        return a.ok!WitString;
    }
    
    @witExport
    Result!(float, E) enumError(float a) {
        if (a == 0.0) {
            return E.a.err!float;
        }
    
        return a.ok!E;
    }
    
    @witExport
    Result!(float, E2) recordError(float a) {
        if (a == 0.0) {
            return E2(
                line: 420,
                column: 0
            ).err!float;
        } else if (a == 1.0) {
            return E2(
                line: 77,
                column: 2
            ).err!float;
        }
    
        return a.ok!E2;
    }
    
    @witExport
    Result!(float, E3) variantError(float a) {
        if (a == 0.0) {
            return E3.e2(E2(
                line: 420,
                column: 0
            )).err!float;
        } else if (a == 1.0) {
            return E3.e1(E.b).err!float;
        } else if (a == 2.0) {
            return E3.e1(E.c).err!float;
        }
    
        return a.ok!E3;
    }
    
    @witExport
    Result!(uint, void) emptyError(uint a) {
        if (a == 0) {
            return err!uint;
        } else if (a == 1) {
            return 42u.ok!void;
        }
    
        return a.ok!void;
    }
    
    @witExport
    Result!(Result!(void, WitString), WitString) doubleError(uint a) {
        if (a == 0) {
            return ok!WitString.ok!WitString;
        } else if (a == 1) {
            return "one".witList.witClone.err!void.ok!WitString;
        }
    
        return "two".witList.witClone.err!(Result!(void, WitString));
    }
}

alias Exports = wit.test.results.leaf.Exports!(
    stringError,
    enumError,
    recordError,
    variantError,
    emptyError,
    doubleError
);
