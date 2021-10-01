use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wasmer::WasmerEnv;

wit_bindgen_wasmer::export!("./tests/runtime/smoke/imports.wit");

#[derive(WasmerEnv, Clone)]
pub struct MyImports {
    hit: Arc<AtomicBool>,
}

impl imports::Imports for MyImports {
    fn thunk(&mut self) {
        self.hit.store(true, Ordering::Relaxed);
        println!("in the host");
    }
}

wit_bindgen_wasmer::import!("./tests/runtime/smoke/exports.wit");

fn run(wasm: &str) -> Result<()> {
    let hit = Arc::new(AtomicBool::new(false));
    let exports = crate::instantiate(
        wasm,
        |store, import_object| {
            imports::add_to_imports(store, import_object, MyImports { hit: hit.clone() })
        },
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    exports.thunk()?;

    assert!(hit.load(Ordering::Relaxed));

    Ok(())
}
