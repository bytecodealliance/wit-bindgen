import wit.my.strings.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    foo("hello".witList);

    auto str = bar();
    scope(exit) str.witFree;

    assert(str == "world");
}

alias Exports = wit.my.strings.runner.Exports!(
    run
);
