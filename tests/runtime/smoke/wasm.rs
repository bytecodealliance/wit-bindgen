wit_bindgen_guest_rust::generate!({
    import: "../../tests/runtime/smoke/imports.wit",
    default: "../../tests/runtime/smoke/exports.wit",
    name: "exports",
});

struct Exports;

impl exports::Exports for Exports {
    fn thunk() {
        imports::thunk();
    }
}
