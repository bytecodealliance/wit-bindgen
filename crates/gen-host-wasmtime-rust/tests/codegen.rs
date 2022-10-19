#![allow(dead_code)]

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        mod $name {
            mod export {
                wit_bindgen_host_wasmtime_rust::generate!({
                    export: $test,
                    name: "the-world-name",

                });
                #[test]
                fn works() {}
            }

            mod import {
                wit_bindgen_host_wasmtime_rust::generate!({
                    import: $test,
                    name: "the-world-name",
                });

                #[test]
                fn works() {}
            }

            mod default {
                wit_bindgen_host_wasmtime_rust::generate!({
                    default: $test,
                    name: "the-world-name",
                });

                #[test]
                fn works() {}
            }

            mod async_ {
                wit_bindgen_host_wasmtime_rust::generate!({
                    import: $test,
                    default: $test,
                    name: "the-world-name",
                    async: true,
                });

                #[test]
                fn works() {}
            }

            mod tracing {
                wit_bindgen_host_wasmtime_rust::generate!({
                    import: $test,
                    default: $test,
                    name: "the-world-name",
                    tracing: true,
                });

                #[test]
                fn works() {}
            }

        }
    };
}

test_helpers::codegen_tests!("*.wit");
