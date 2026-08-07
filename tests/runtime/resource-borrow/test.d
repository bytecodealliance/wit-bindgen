import wit.test.resource_borrow.test;
import wit.common;

@witExport("test:resource-borrow/to-test", "thing")
struct ThingImpl {
    uint val;

    @witExport("test:resource-borrow/to-test", "[constructor]thing")
    static Thing constructor(uint v) {
        return Thing.makeNew((out typeof(this) self) {
            self.val = v + 1;
        });
    }
}

@witExport("test:resource-borrow/to-test", "foo")
uint foo(Thing.Borrow v) {
    return v.rep!ThingImpl.val + 2;
}

alias Exports = wit.test.resource_borrow.test.Exports!(
    ThingImpl,
    foo
);
