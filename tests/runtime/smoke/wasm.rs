wai_bindgen_rust::import!("./tests/runtime/smoke/imports.wai");
wai_bindgen_rust::export!("./tests/runtime/smoke/exports.wai");

struct Exports;

impl exports::Exports for Exports {
    fn thunk() {
        println!("in the wasm");
        imports::thunk();
    }
}
