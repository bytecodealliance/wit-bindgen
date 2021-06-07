#![allow(dead_code, type_alias_bounds)]

fn main() {
    println!("compiled successfully!")
}

#[rustfmt::skip]
mod imports {
    test_codegen::wasmtime_import!(
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
    test_codegen::wasmtime_export!(
        "*.witx"

        // This uses preview1 ABI things which are only supported for imports
        "!host.witx"

        // If you want to exclude other test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.witx"
    );
}
