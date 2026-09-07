module leaf_thing;

import wit.test.resource_import_and_export.leaf_thing;

import wit.common;

@witInterface("test:resource-import-and-export/test")
@witExport("thing")
struct ThingImpl {
    uint val;

    @witExport("[constructor]")
    static Thing constructor(uint v) {
        return Thing.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }

    @witExport("foo")
    uint foo() {
        return val + 2;
    }

    @witExport("bar")
    void bar(uint v) {
        val = v + 3;
    }

    @witExport("baz")
    static Thing baz(Thing a, Thing b) {
        return ThingImpl.constructor(
            a.rep!ThingImpl.foo + b.rep!ThingImpl.foo + 4
        );
    }
}

alias Exports = wit.test.resource_import_and_export.leaf_thing.Exports!(
    ThingImpl
);
