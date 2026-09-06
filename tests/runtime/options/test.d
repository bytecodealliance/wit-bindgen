import wit.test.options.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witInterface("test:options/to-test") {
    @witExport("option-none-param")
    void optionNoneParam(in Option!WitString a) {
    }
    
    @witExport("option-some-param")
    void optionSomeParam(in Option!WitString a) {
    }
    
    @witExport("option-none-result")
    Option!WitString optionNoneResult() {
        return none!WitString;
    }
    
    @witExport("option-some-result")
    Option!WitString optionSomeResult() {
        return "foo".witList.witClone.some;
    }
    
    @witExport("option-roundtrip")
    Option!WitString optionRoundtrip(in Option!WitString a) {
        return a.witClone;
    }
    @witExport("double-option-roundtrip")
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
