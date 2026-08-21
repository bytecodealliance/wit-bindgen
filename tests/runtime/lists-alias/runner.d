import wit.my.lists.runner;
import cat = wit.my.lists.runner.imports.cat;
import wit.common;

@witExport("$root", "run")
void run() {
    cat.foo((cast(immutable ubyte[])"hello").witList);

    WitList!ubyte t = cat.bar();
    scope(exit) t.witFree;
    assert(t == (cast(immutable ubyte[])"world").witList);
}

alias Exports = wit.my.lists.runner.Exports!(
    run
);
