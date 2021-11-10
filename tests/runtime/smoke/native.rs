use anyhow::Result;

wai_bindgen_rust::export!("./tests/runtime/smoke/imports.wai");

struct Imports;

impl imports::Imports for Imports {
    fn thunk() {
        println!("in the host");
    }
}

wai_bindgen_rust::import!("./tests/runtime/smoke/exports.wai");

fn run() -> Result<()> {
    exports::thunk();
    Ok(())
}
