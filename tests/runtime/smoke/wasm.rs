wit_bindgen_guest_rust::generate!("../../tests/runtime/smoke/world.wit");

struct Exports;

export_smoke!(Exports);

impl smoke::Smoke for Exports {
    fn thunk() {
        imports::thunk();
    }
}
