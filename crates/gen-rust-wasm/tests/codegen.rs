#![allow(dead_code, type_alias_bounds)]

#[test]
fn ok() {}

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_rust_wasm_import!(
        "*.witx"

        // If you want to exclude a specific test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.witx"
        // "!host.witx"
        //
        //
        // Similarly you can also just remove the `*.witx` glob and list tests
        // individually if you're debugging.
    );
}

mod exports {
    test_helpers::codegen_rust_wasm_export!(
        "*.witx"

        // TODO: these use push/pull buffer which isn't implemented in the test
        // generator just yet
        "!wasi_next.witx"
        "!host.witx"

        // These use the preview1 ABI which isn't implemented for rust_wasm exports.
        "!wasi_snapshot_preview1.witx"
        "!typenames.witx"
        "!legacy.witx"
    );
}
