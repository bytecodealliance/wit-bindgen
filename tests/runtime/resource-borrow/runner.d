import wit.test.resource_borrow.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    auto thing = Thing.makeNew(42);
    scope(exit) thing.witDrop;

    assert(foo(thing) == 42 + 1 + 2);
}

alias Exports = wit.test.resource_borrow.runner.Exports!(
    run
);
