module leaf_thing;

import wit.test.resource_import_and_export.leaf_thing;

import wit.common;

@witExport("test:resource-import-and-export/test", "thing")
struct ThingImpl {
    uint val;

    @witExport("test:resource-import-and-export/test", "[constructor]thing")
    static Thing constructor(uint v) {
        return Thing.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }

    @witExport("test:resource-import-and-export/test", "[method]thing.foo")
    uint foo() {
        return val + 2;
    }

    @witExport("test:resource-import-and-export/test", "[method]thing.bar")
    void bar(uint v) {
        val = v + 3;
    }

    @witExport("test:resource-import-and-export/test", "[static]thing.baz")
    static Thing baz(Thing a, Thing b) {
        return ThingImpl.constructor(
            a.rep!ThingImpl.foo + b.rep!ThingImpl.foo + 4
        );
    }
}

alias Exports = wit.test.resource_import_and_export.leaf_thing.Exports!(
    ThingImpl
);
