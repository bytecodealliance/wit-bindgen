import wit.test.resource_borrow_in_record.test;

import wit.common;

extern(C) void* malloc(size_t size);
extern(C) void free(void* ptr);

char[] concat(in char[] a, in char[] b) {
    auto ptr = cast(char*)malloc(a.length + b.length);
    assert((a.length + b.length) == 0 || ptr);

    if (!ptr) return null;

    ptr[0..a.length] = a[];
    ptr[a.length..a.length+b.length] = b[];

    return ptr[0..a.length+b.length];
}

@witExport("test:resource-borrow-in-record/to-test", "thing")
struct ThingImpl {
    const(char)[] contents;

    @witExport("test:resource-borrow-in-record/to-test", "[constructor]thing")
    static Thing constructor(in WitString v) {
        return Thing.makeNew((out typeof(this) self) {
            self.contents = concat(v, " new");
        });
    }

    @witExport("test:resource-borrow-in-record/to-test", "[method]thing.get")
    WitString get() {
        return concat(contents, " get").witList;
    }
}

@witExport("test:resource-borrow-in-record/to-test", "test")
WitList!Thing test(ref scope WitList!Foo list) {
    if (list.length == 0) return WitList!Thing();

    auto ptr = cast(Thing*)malloc(Thing.sizeof*list.length);
    assert(ptr);

    auto things = ptr[0..list.length];

    foreach (i, ref thing; things) {
        auto orig = list[i].thing.rep!ThingImpl.contents;
        auto str = concat(orig, " test");

        thing = Thing.makeNew((out ThingImpl self) {
            self.contents = str;
        });
    }

    return things.witList; // already on C heap; no clone
}

alias Exports = wit.test.resource_borrow_in_record.test.Exports!(
    ThingImpl,
    test
);
