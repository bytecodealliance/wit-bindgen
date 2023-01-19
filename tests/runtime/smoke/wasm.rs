wit_bindgen_guest_rust::generate!("world" in "../../tests/runtime/smoke");

struct Exports;

export_smoke!(Exports);

impl Smoke for Exports {
    fn thunk() {
        imports::thunk();
    }
}
