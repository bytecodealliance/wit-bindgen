#![allow(dead_code)]

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

        // These use preview1 ABI things which are only supported for imports
        "!host.witx"
        "!wasi_snapshot_preview1.witx"

        // If you want to exclude other test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.witx"
    );
}
