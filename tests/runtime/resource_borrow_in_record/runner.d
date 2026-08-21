import wit.test.resource_borrow_in_record.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    auto thing1 = Thing.makeNew("Bonjour".witList);
    scope(exit) thing1.witDrop;

    auto thing2 = Thing.makeNew("mon cher".witList);
    scope(exit) thing2.witDrop;

    Foo[2] things = [
        Foo(thing1),
        Foo(thing2)
    ];

    auto result = test(things.witList);
    scope(exit) {
        result.witDrop;
        result.witFree;
    }

    assert(result.length == 2);
    assert(result[0].get == "Bonjour new test get");
    assert(result[1].get == "mon cher new test get");
}

alias Exports = wit.test.resource_borrow_in_record.runner.Exports!(
    run
);
