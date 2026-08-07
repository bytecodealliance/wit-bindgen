import wit.test.resource_import_and_export.intermediate;
import wit.test.resource_import_and_export.test.imports : ThingImport = Thing;
import wit.test.resource_import_and_export.test.exports : ThingExport = Thing;

import wit.common;

@witExport("test:resource-import-and-export/test", "thing")
struct ThingImpl {
    ThingImport thing;

    @witExport("test:resource-import-and-export/test", "[constructor]thing")
    static ThingExport constructor(uint v) {
        return ThingExport.makeNew((out typeof(this) self) {
            self.thing = ThingImport.makeNew(v + 1);
        });
    }

    @witExport("test:resource-import-and-export/test", "[method]thing.foo")
    uint foo() {
        return thing.foo + 2;
    }

    @witExport("test:resource-import-and-export/test", "[method]thing.bar")
    void bar(uint v) {
        thing.bar(v + 3);
    }

    @witExport("test:resource-import-and-export/test", "[static]thing.baz")
    static ThingExport baz(ThingExport a, ThingExport b) {
        scope(exit) {
            a.witDrop;
            b.witDrop;
        }

        auto aRep = a.rep!ThingImpl;
        auto bRep = b.rep!ThingImpl;

        auto result = ThingImport.baz(
            aRep.thing,
            bRep.thing
        );
        aRep.thing = Thing.init; // consumed by `baz`
        bRep.thing = Thing.init; // consumed by `baz`
        scope(exit) result.witDrop;

        return ThingImpl.constructor(result.foo + 4);
    }
}

@witExport("$root", "toplevel-export")
ThingImport toplevelExport(ThingImport input) {
    // `input` not dropped b/c ownership transferred
    // to `toplevelImport`

    return toplevelImport(input);
}

alias Exports = wit.test.resource_import_and_export.intermediate.Exports!(
    ThingImpl,
    toplevelExport
);
