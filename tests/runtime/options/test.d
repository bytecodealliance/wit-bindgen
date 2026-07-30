import wit.test.options.test;
import wit.common;

import std.meta : Repeat, AliasSeq;

@witExport("test:options/to-test", "option-none-param")
void optionNoneParam(in Option!WitString a) {
}

@witExport("test:options/to-test", "option-some-param")
void optionSomeParam(in Option!WitString a) {
}

@witExport("test:options/to-test", "option-none-result")
Option!WitString optionNoneResult() {
    return none!WitString;
}

@witExport("test:options/to-test", "option-some-result")
Option!WitString optionSomeResult() {
    return "foo".witList.witClone.some;
}

@witExport("test:options/to-test", "option-roundtrip")
Option!WitString optionRoundtrip(in Option!WitString a) {
    return a.witClone;
}
@witExport("test:options/to-test", "double-option-roundtrip")
Option!(Option!uint) doubleOptionRoundtrip(in Option!(Option!uint) a) {
    return a.witClone;
}


alias Exports = wit.test.options.test.Exports!(
    optionNoneParam,
    optionSomeParam,
    optionNoneResult,
    optionSomeResult,

    optionRoundtrip,
    doubleOptionRoundtrip
);
