import wit.test.resource_with_lists.leaf;

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
    ubyte[] val;

    @witExport("test:resource-with-lists/test", "[constructor]thing")
    static Thing constructor(in WitList!ubyte a) {
        return Thing.makeNew((out typeof(this) self) {
            auto result = a.concat(" HostThing");

            self.val = result;
        });
    }

    @witExport("test:resource-with-lists/test", "[method]thing.foo")
    WitList!ubyte foo() {
        auto result = val.concat(" HostThing.foo");
        return result.witList; // no clone, no free; already on C heap
    }

    @witExport("test:resource-with-lists/test", "[method]thing.bar")
    auto bar(in WitList!ubyte l) {
        auto result = l.concat(" HostThing.bar");

        if (val.ptr) free(val.ptr);
        val = result;
    }


    @witExport("test:resource-with-lists/test", "[static]thing.baz")
    static WitList!ubyte baz(in WitList!ubyte l) {
        auto result = l.concat(" HostThing.baz");
        return result.witList; // no clone, no free; already on C heap
    }

    ~this() {
        if (val.ptr) free(val.ptr);
    }
}

alias Exports = wit.test.resource_with_lists.leaf.Exports!(
    ThingImpl
);
