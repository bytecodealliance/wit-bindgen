import wit.test.resource_borrow.test;
import wit.common;


@witInterface("test:resource-borrow/to-test") {
    @witExport("thing")
    struct ThingImpl {
        uint val;
    
        @witExport("[constructor]")
        static Thing constructor(uint v) {
            return Thing.makeNew((out typeof(this) self) {
                self.val = v + 1;
            });
        }
    }
    
    @witExport("foo")
    uint foo(Thing.Borrow v) {
        return v.rep!ThingImpl.val + 2;
    }
}

alias Exports = wit.test.resource_borrow.test.Exports!(
    ThingImpl,
    foo
);
