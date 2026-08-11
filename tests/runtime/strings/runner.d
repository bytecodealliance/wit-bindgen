import wit.test.strings.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    takeBasic("latin utf16".witList);

    {
        auto str = returnUnicode();
        scope(exit) str.witFree;

        assert(str == "🚀🚀🚀 𠈄𓀀");
    }

    {
        auto str = returnEmpty();
        scope(exit) str.witFree;

        assert(str == "");
    }

    {
        auto str = roundtrip("🚀🚀🚀 𠈄𓀀".witList);
        scope(exit) str.witFree;

        assert(str == "🚀🚀🚀 𠈄𓀀");
    }
}

alias Exports = wit.test.strings.runner.Exports!(
    run
);
