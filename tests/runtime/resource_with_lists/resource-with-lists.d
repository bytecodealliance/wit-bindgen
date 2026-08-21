module resource_with_lists;

import wit.test.resource_with_lists.resource_with_lists;
import wit.test.resource_with_lists.test.imports : IThing = Thing;
import wit.test.resource_with_lists.test.exports : EThing = Thing;

import wit.common;

extern(C) void* malloc(size_t size);
extern(C) void free(void* ptr);

ubyte[] concat(in ubyte[] a, in char[] b) {
    auto ptr = cast(ubyte*)malloc(a.length + b.length);
    assert((a.length + b.length) == 0 || ptr);

    if (!ptr) return null;

    ptr[0..a.length] = a[];
    ptr[a.length..a.length+b.length] = cast(const(ubyte[]))b[];

    return ptr[0..a.length+b.length];
}

@witExport("test:resource-with-lists/test", "thing")
struct ThingImpl {
    IThing val;

    @witExport("test:resource-with-lists/test", "[constructor]thing")
    static EThing constructor(in WitList!ubyte a) {
        return EThing.makeNew((out typeof(this) self) {
            auto result = a.concat(" Thing");
            scope(exit) free(result.ptr);

            self.val = IThing.makeNew(result.witList);
        });
    }

    @witExport("test:resource-with-lists/test", "[method]thing.foo")
    WitList!ubyte foo() {
        auto list = val.foo;
        scope(exit) list.witFree;

        auto result = list.concat(" Thing.foo");
        return result.witList; // no clone, no free; already on C heap
    }

    @witExport("test:resource-with-lists/test", "[method]thing.bar")
    auto bar(in WitList!ubyte l) {
        auto result = l.concat(" Thing.bar");
        scope(exit) free(result.ptr);

        val.bar(result.witList);
    }


    @witExport("test:resource-with-lists/test", "[static]thing.baz")
    static WitList!ubyte baz(in WitList!ubyte l) {
        auto input = l.concat(" Thing.baz");
        scope(exit) free(input.ptr);

        auto impBaz = IThing.baz(input.witList);
        scope(exit) impBaz.witFree;

        auto result = impBaz.concat(" Thing.baz again");
        return result.witList;
    }

    ~this() {
        val.witDrop;
    }
}

alias Exports = wit.test.resource_with_lists.resource_with_lists.Exports!(
    ThingImpl
);
