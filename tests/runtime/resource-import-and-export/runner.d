import wit.test.resource_import_and_export.runner;
import wit.common;

@witExport("$root", "run")
void run() {
    auto thing1 = Thing.makeNew(42);
    scope(exit) thing1.witDrop;

    // 42 + 1 (constructor) + 1 (constructor) + 2 (foo) + 2 (foo)
    assert(thing1.foo == 48);

    // 33 + 3 (bar) + 3 (bar) + 2 (foo) + 2 (foo)
    thing1.bar(33);
    assert(thing1.foo() == 43);

    auto thing2 = Thing.makeNew(81);
    scope(exit) thing2.witDrop;

    auto thing3 = Thing.baz(thing1, thing2);
    thing1 = Thing.init; // thing1 consumed by `baz`
    thing2 = Thing.init; // thing2 consumed by `baz`
    scope(exit) thing3.witDrop;
}

alias Exports = wit.test.resource_import_and_export.runner.Exports!(
    run
);
