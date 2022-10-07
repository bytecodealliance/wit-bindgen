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
                wit_bindgen_host_wasmtime_rust::import!({
                    paths: [$test],
                    custom_error: true,
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
}

mod custom_errors {
    wit_bindgen_host_wasmtime_rust::export!({
        src["x"]: "
            foo: func()
            bar: func() -> result<_, u32>
            enum errno {
                bad1,
                bad2,
            }
            baz: func() -> result<u32, errno>
        ",
        custom_error: true,
    });
}
