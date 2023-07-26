wit_bindgen::generate!({
    path: "../../tests/runtime/smoke",
    exports: {
        world: Exports
    }
});

struct Exports;

impl Smoke for Exports {
    fn thunk() {
        test::smoke::imports::thunk();
    }
}
