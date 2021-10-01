#![allow(dead_code, type_alias_bounds)]

fn main() {
    println!("compiled successfully!")
}

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_wasmer_export!(
        "*.wit"
        "*.witx"
 
        // TODO: implement async support
        "!async_functions.wit"

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
    test_helpers::codegen_wasmer_import!(
        "*.wit"
 
        // TODO: implement async support
        "!async_functions.wit"

        // TODO: these use push/pull buffer which isn't implemented in the test
        // generator just yet
        "!wasi_next.wit"
        "!host.wit"
    );
}

/*
mod async_tests {
    mod not_async {
        wit_bindgen_wasmer::export!({
            src["x"]: "foo: function()",
            async: ["bar"],
        });

        struct Me;

        impl x::X for Me {
            fn foo(&mut self) {}
        }
    }
    mod one_async {
        wit_bindgen_wasmer::export!({
            src["x"]: "
                foo: function() -> list<u8>
                bar: function()
            ",
            async: ["bar"],
        });

        struct Me;

        #[wit_bindgen_wasmer::async_trait]
        impl x::X for Me {
            fn foo(&mut self) -> Vec<u8> {
                Vec::new()
            }

            async fn bar(&mut self) {}
        }
    }
    mod one_async_export {
        wit_bindgen_wasmer::import!({
            src["x"]: "
                foo: function(x: list<string>)
                bar: function()
            ",
            async: ["bar"],
        });
    }
    mod resource_with_none_async {
        wit_bindgen_wasmer::export!({
            src["x"]: "
                resource y {
                    z: function() -> string
                }
            ",
            async: [],
        });
    }
}
*/

mod custom_errors {
    wit_bindgen_wasmer::export!({
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
