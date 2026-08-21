import wit.test.resource_alias_redux.runner;

import wit.test.resource_alias_redux.runner.imports.the_test : Thing1 = Thing;
import wit.test.resource_alias_redux.resource_alias1.imports : Foo1 = Foo, Thing2 = Thing;
import wit.test.resource_alias_redux.resource_alias2.imports : Foo2 = Foo;

import wit.common;

@witExport("$root", "run")
void run() {
    auto thing1 = Thing.makeNew("Ni Hao".witList);
    scope(exit) thing1.witDrop;

    {
        Thing1[1] things = [thing1];
        auto result = test(things.witList);
        thing1 = Thing1.init; // consumed
        scope(exit) result.witFree;
        assert(result.length == 1);

        {
            auto str = result[0].get;
            scope(exit) str.witFree;

            assert(str == "Ni Hao GuestThing GuestThing.get");
        }
    }


    auto thing2 = Thing2.makeNew("Ciao".witList);
    scope(exit) thing2.witDrop;

    {
        auto result = a(Foo1(thing: thing2));
        thing2 = Thing2.init; // consumed
        scope(exit) result.witFree;
        assert(result.length == 1);

        {
            auto str = result[0].get;
            scope(exit) str.witFree;

            assert(str == "Ciao GuestThing GuestThing.get");
        }
    }


    auto thing3 = Thing2.makeNew("Ciao".witList);
    scope(exit) thing3.witDrop;
    auto thing4 = Thing2.makeNew("Aloha".witList);
    scope(exit) thing4.witDrop;

    {
        auto result = b(Foo2(thing: thing3), Bar(thing: thing4));
        thing3 = Thing2.init; // consumed
        thing4 = Thing2.init; // consumed
        scope(exit) result.witFree;
        assert(result.length == 2);

        {
            auto str = result[0].get;
            scope(exit) str.witFree;

            assert(str == "Ciao GuestThing GuestThing.get");
        }

        {
            auto str = result[1].get;
            scope(exit) str.witFree;

            assert(str == "Aloha GuestThing GuestThing.get");
        }
    }
}

alias Exports = wit.test.resource_alias_redux.runner.Exports!(
    run
);
