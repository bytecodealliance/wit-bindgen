#![allow(dead_code)]

macro_rules! codegen_test {
    ($name:ident $test:tt) => {
        mod $name {
            mod default {
                wit_bindgen_host_wasmtime_rust::generate!($test);

                #[test]
                fn works() {}
            }

            mod async_ {
                wit_bindgen_host_wasmtime_rust::generate!({
                    path: $test,
                    async: true,
                });

                #[test]
                fn works() {}
            }

            mod tracing {
                wit_bindgen_host_wasmtime_rust::generate!({
                    path: $test,
                    tracing: true,
                });

                #[test]
                fn works() {}
            }

        }
    };
}

test_helpers::codegen_tests!("*.wit");
