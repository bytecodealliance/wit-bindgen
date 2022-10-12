#![allow(dead_code)]

mod exports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            wit_bindgen_host_wasmtime_rust::export!($test);

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");

    mod with_options {
        macro_rules! codegen_test {
            ($name:ident $test:tt) => {
                wit_bindgen_host_wasmtime_rust::export!({
                    paths: [$test],
                    tracing: true,
                });

                #[test]
                fn $name() {}
            };
        }
        test_helpers::codegen_tests!("*.wit");
    }
}

mod imports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            wit_bindgen_host_wasmtime_rust::import!($test);

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");

    mod with_options {
        macro_rules! codegen_test {
            ($name:ident $test:tt) => {
                wit_bindgen_host_wasmtime_rust::import!({
                    paths: [$test],
                    tracing: true,
                });

                #[test]
                fn $name() {}
            };
        }
        test_helpers::codegen_tests!("*.wit");
    }
}
