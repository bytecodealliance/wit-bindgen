use crate::validation::ValidatedAdapter;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use wasmparser::FuncType;
use wit_parser::Interface;

mod gc;

pub fn adapt<'a>(
    wasm: &[u8],
    interface: &'a Interface,
    required: &IndexMap<&str, FuncType>,
) -> Result<(Vec<u8>, ValidatedAdapter<'a>)> {
    let wasm = gc::run(wasm, required)
        .context("failed to reduce input adapter module to its minimal size")?;
    let info = crate::validation::validate_adapter_module(&wasm, interface, required)
        .context("failed to validate the imports of the minimized adapter module")?;
    Ok((wasm, info))
}
