#![allow(dead_code, type_alias_bounds)]

#[test]
fn ok() {}

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_rust_wasm_import!(
        "*.wit"

        // If you want to exclude a specific test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.wit"
        // "!host.wit"
        //
        //
        // Similarly you can also just remove the `*.wit` glob and list tests
        // individually if you're debugging.
    );
}

mod exports {
    test_helpers::codegen_rust_wasm_export!(
        "*.wit"

        // TODO: these use push/pull buffer which isn't implemented in the test
        // generator just yet
        "!wasi-next.wit"
        "!host.wit"
    );
}
