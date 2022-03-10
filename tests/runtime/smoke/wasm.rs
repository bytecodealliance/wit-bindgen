wit_bindgen_rust::import!("../../tests/runtime/smoke/imports.wit");
wit_bindgen_rust::export!("../../tests/runtime/smoke/exports.wit");

struct Exports;

impl exports::Exports for Exports {
    fn thunk() {
        imports::thunk();
    }
}
