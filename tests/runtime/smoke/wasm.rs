wit_bindgen::generate!(in "../../tests/runtime/smoke");

struct Exports;

export_smoke!(Exports);

impl Smoke for Exports {
    fn thunk() {
        test::smoke::imports::thunk();
    }
}
