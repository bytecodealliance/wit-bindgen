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

        // These use preview1 ABI things which are only supported for imports
        "!host.witx"
        "!typenames.witx"
        "!wasi_snapshot_preview1.witx"

        // This uses buffers, which we don't support in exports just yet
        // TODO: should support this
        "!wasi_next.witx"
    );
}

mod async_tests {
    mod not_async {
        witx_bindgen_wasmtime::import!({
            src["x"]: "foo: function()",
            async: ["bar"],
        });

        struct Me;

        impl x::X for Me {
            fn foo(&mut self) {}
        }
    }
    mod one_async {
        witx_bindgen_wasmtime::import!({
            src["x"]: "
                foo: function() -> list<u8>
                bar: function()
            ",
            async: ["bar"],
        });

        struct Me;

        #[witx_bindgen_wasmtime::async_trait]
        impl x::X for Me {
            fn foo(&mut self) -> Vec<u8> {
                Vec::new()
            }

            async fn bar(&mut self) {}
        }
    }
    mod one_async_export {
        witx_bindgen_wasmtime::export!({
            src["x"]: "
                foo: function(x: list<string>)
                bar: function()
            ",
            async: ["bar"],
        });
    }
    mod resource_with_none_async {
        witx_bindgen_wasmtime::import!({
            src["x"]: "
                resource y {
                    z: function() -> string
                }
            ",
            async: [],
        });
    }
}

mod custom_errors {
    witx_bindgen_wasmtime::import!({
        src["x"]: "
            foo: function()
            bar: function() -> expected<_, u32>
            enum errno {
                bad1,
                bad2,
            }
            baz: function() -> expected<u32, errno>
        ",
        custom_error: true,
    });
}
