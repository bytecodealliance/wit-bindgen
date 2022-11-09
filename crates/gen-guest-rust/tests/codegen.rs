#![allow(unused_macros)]

mod exports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            mod $name {
                wit_bindgen_guest_rust::generate!({
                    export: $test,
                    name: "not-used-name",
                });

                mod default {
                    wit_bindgen_guest_rust::generate!({
                        default: $test,
                        name: "the-world-name",
                    });

                    #[test]
                    fn $name() {}
                }
            }

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");
}

mod imports {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            wit_bindgen_guest_rust::generate!({
                import: $test,
                name: "not-used-name",
            });

            #[test]
            fn $name() {}
        };
    }
    test_helpers::codegen_tests!("*.wit");

    mod unchecked {
        macro_rules! codegen_test {
            ($name:ident $test:tt) => {
                wit_bindgen_guest_rust::generate!({
                    import: $test,
                    unchecked,
                    name: "not-used-name",
                });

                #[test]
                fn $name() {}
            };
        }
        test_helpers::codegen_tests!("*.wit");
    }
}

mod altogether {
    macro_rules! codegen_test {
        ($name:ident $test:tt) => {
            mod $name {
                wit_bindgen_guest_rust::generate!({
                    // rename the input `*.wit` file for imports/exports to
                    // avoid having them having the same name which the rust
                    // generator currently doesn't support.
                    import["the-import"]: $test,
                    export["the-export"]: $test,
                    default: $test,
                    unchecked,
                    name: "not-used-name",
                });

                #[test]
                fn works() {}
            }
        };
    }
    test_helpers::codegen_tests!("*.wit");
}

mod strings {
    wit_bindgen_guest_rust::generate!({
        import_str["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
        name: "not-used-name",
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
    wit_bindgen_guest_rust::generate!({
        import_str["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
        raw_strings,
        name: "not-used-name",
    });

    #[allow(dead_code)]
    fn test() {
        // Test the argument is `&[u8]`.
        cat::foo(b"hello");

        // Test the return type is `Vec<u8>`.
        let _t: Vec<u8> = cat::bar();
    }
}

// This is a static compilation test to ensure that
// export bindings can go inside of another mod/crate
// and still compile.
mod prefix {
    mod bindings {
        wit_bindgen_guest_rust::generate!({
            export_str["exports1"]: "
                foo: func(x: string)
                bar: func() -> string
            ",
            name: "baz",
            macro_call_prefix: "bindings::"
        });

        pub(crate) use export_baz;
    }

    struct Component;

    impl bindings::exports1::Exports1 for Component {
        fn foo(x: String) {
            println!("foo: {}", x);
        }

        fn bar() -> String {
            "bar".to_string()
        }
    }

    bindings::export_baz!(Component);
}

// This is a static compilation test to check that
// the export macro name can be overridden.
mod macro_name {
    wit_bindgen_guest_rust::generate!({
        export_str["exports2"]: "
            foo: func(x: string)
        ",
        name: "baz",
        export_macro_name: "jam"
    });

    struct Component;

    impl exports2::Exports2 for Component {
        fn foo(x: String) {
            println!("foo: {}", x);
        }
    }

    jam!(Component);
}

mod skip {
    wit_bindgen_guest_rust::generate!({
        export_str["exports"]: "
            foo: func()
            bar: func()
        ",
        name: "baz",
        skip: ["foo"],
    });

    struct Component;

    impl exports::Exports for Component {
        fn bar() {}
    }

    export_baz!(Component);
}
