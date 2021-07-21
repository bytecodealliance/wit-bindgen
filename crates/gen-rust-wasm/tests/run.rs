#![allow(dead_code, type_alias_bounds)]

fn main() {
    println!("compiled successfully!")
}

#[rustfmt::skip]
mod imports {
    test_codegen::rust_wasm_import!(
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
    test_codegen::rust_wasm_export!(
        "*.witx"

        // TODO: these use push/pull buffer which isn't implemented in the test
        // generator just yet
        "!wasi_next.witx"
        "!host.witx"
    );
}
