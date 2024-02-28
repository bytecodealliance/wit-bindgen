wit_bindgen::generate!({
    path: "../../tests/runtime/smoke",
});

struct Exports;

export!(Exports);

impl Guest for Exports {
    fn thunk() {
        test::smoke::imports::thunk();
    }
}
