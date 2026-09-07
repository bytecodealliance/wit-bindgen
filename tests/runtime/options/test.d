import wit.test.options.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witInterface("test:options/to-test") {
    @witExport
    void optionNoneParam(in Option!WitString a) {
    }
    
    @witExport
    void optionSomeParam(in Option!WitString a) {
    }
    
    @witExport
    Option!WitString optionNoneResult() {
        return none!WitString;
    }
    
    @witExport
    Option!WitString optionSomeResult() {
        return "foo".witList.witClone.some;
    }
    
    @witExport
    Option!WitString optionRoundtrip(in Option!WitString a) {
        return a.witClone;
    }
    @witExport
    Option!(Option!uint) doubleOptionRoundtrip(in Option!(Option!uint) a) {
        return a.witClone;
    }
}

alias Exports = wit.test.options.test.Exports!(
    optionNoneParam,
    optionSomeParam,
    optionNoneResult,
    optionSomeResult,

    optionRoundtrip,
    doubleOptionRoundtrip
);
