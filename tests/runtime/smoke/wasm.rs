witx_bindgen_rust::import!("./tests/runtime/smoke/imports.witx");
witx_bindgen_rust::export!("./tests/runtime/smoke/exports.witx");

struct Exports;

impl exports::Exports for Exports {
    fn thunk() {
        imports::thunk();
    }
}
