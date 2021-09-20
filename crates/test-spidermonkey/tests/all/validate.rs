use anyhow::{Context, Result};
use test_spidermonkey::{generate, witx_files};
use witx_bindgen_gen_core::witx2;

fn validate(wasm: &[u8]) -> Result<()> {
    let mut validator = wasmparser::Validator::new();
    validator.wasm_features(wasmparser::WasmFeatures {
        bulk_memory: true,
        module_linking: true,
        multi_memory: true,
        ..wasmparser::WasmFeatures::default()
    });
    validator.validate_all(wasm)?;
    Ok(())
}

/// Test that we generate valid Wasm glue files when treating each of our test
/// `*.witx` files as an import.
#[test]
fn import() -> anyhow::Result<()> {
    for witx in witx_files()? {
        let witx = witx?;
        let wasm = generate(&witx, witx2::abi::Direction::Import)?;
        validate(&wasm)
            .with_context(|| format!("generated wasm for {} is invalid", witx.display()))?;
    }
    Ok(())
}

/// Test that we generate valid Wasm glue files when treating each of our test
/// `*.witx` files as an export.
#[test]
fn export() -> anyhow::Result<()> {
    for witx in witx_files()? {
        let witx = witx?;
        let wasm = generate(&witx, witx2::abi::Direction::Export)?;
        validate(&wasm)
            .with_context(|| format!("generated wasm for {} is invalid", witx.display()))?;
    }
    Ok(())
}
