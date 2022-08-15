#![allow(dead_code, type_alias_bounds)]

fn main() {
    println!("compiled successfully!")
}

#[rustfmt::skip]
mod exports {
    test_helpers::codegen_wasmtime_export!(
        "*.wit"

        // TODO: implement async support
        "!async-functions.wit"

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

mod imports {
    test_helpers::codegen_wasmtime_import!(
        "*.wit"

        // TODO: implement async support
        "!async-functions.wit"

        // TODO: these use push/pull buffer which isn't implemented in the test
        // generator just yet
        "!wasi-next.wit"
        "!host.wit"
    );
}

mod async_tests {
    mod not_async {
        wit_bindgen_host_wasmtime_rust::export!({
            src["x"]: "foo: func()",
            async: ["bar"],
        });

        struct Me;

        impl x::X for Me {
            fn foo(&mut self) {}
        }
    }
    mod one_async {
        wit_bindgen_host_wasmtime_rust::export!({
            src["x"]: "
                foo: func() -> list<u8>
                bar: func()
            ",
            async: ["bar"],
        });

        struct Me;

        #[wit_bindgen_host_wasmtime_rust::async_trait]
        impl x::X for Me {
            fn foo(&mut self) -> Vec<u8> {
                Vec::new()
            }

            async fn bar(&mut self) {}
        }
    }
    mod one_async_export {
        wit_bindgen_host_wasmtime_rust::import!({
            src["x"]: "
                foo: func(x: list<string>)
                bar: func()
            ",
            async: ["bar"],
        });
    }
    mod resource_with_none_async {
        wit_bindgen_host_wasmtime_rust::export!({
            src["x"]: "
                resource y {
                    z: func() -> string
                }
            ",
            async: [],
        });
    }
}

mod custom_errors {
    wit_bindgen_host_wasmtime_rust::export!({
        src["x"]: "
            foo: func()
            bar: func() -> expected<unit, u32>
            enum errno {
                bad1,
                bad2,
            }
            baz: func() -> expected<u32, errno>
        ",
        custom_error: true,
    });
}
