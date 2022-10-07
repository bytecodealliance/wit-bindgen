mod exports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            wit_bindgen_guest_rust::export!($test);

            guest_rust_test_macro::gen_dummy_export!($test);

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");
}

mod imports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            wit_bindgen_guest_rust::import!($test);

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");

    mod unchecked {
        macro_rules! codegen_test {
            ($name:ident $test:tt) => {
                wit_bindgen_guest_rust::import!({
                    paths: [$test],
                    unchecked,
                });

                #[test]
                fn $name() {}
            };
        }
        test_helpers::codegen_tests!("*.wit");
    }
}

mod strings {
    wit_bindgen_guest_rust::import!({
        src["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
    });

    #[allow(dead_code)]
    fn test() {
        // Test the argument is `&str`.
        cat::foo("hello");

        // Test the return type is `String`.
        let _t: String = cat::bar();
    }
}

/// Like `strings` but with raw_strings`.
mod raw_strings {
    wit_bindgen_guest_rust::import!({
        src["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
        raw_strings,
    });

    #[allow(dead_code)]
    fn test() {
        // Test the argument is `&[u8]`.
        cat::foo(b"hello");

        // Test the return type is `Vec<u8>`.
        let _t: Vec<u8> = cat::bar();
    }
}
