import wit.test.resource_with_lists.runner;

import wit.common;

@witExport("$root", "run")
void run() {
    auto thingInstance = Thing.makeNew((cast(immutable ubyte[])"Hi").witList);
    scope(exit) thingInstance.witDrop;

    {
        auto result = thingInstance.foo();
        scope(exit) result.witFree;

        assert(cast(char[])result[] == "Hi Thing HostThing HostThing.foo Thing.foo");
    }

    thingInstance.bar((cast(immutable ubyte[])"Hola").witList);

    {
        auto result = thingInstance.foo();
        scope(exit) result.witFree;

        assert(cast(char[])result[] == "Hola Thing.bar HostThing.bar HostThing.foo Thing.foo");
    }

    {
        auto result = thingInstance.baz((cast(immutable ubyte[])"Ohayo Gozaimas").witList);
        scope(exit) result.witFree;

        assert(cast(char[])result[] == "Ohayo Gozaimas Thing.baz HostThing.baz Thing.baz again");
    }
}

alias Exports = wit.test.resource_with_lists.runner.Exports!(
    run
);
