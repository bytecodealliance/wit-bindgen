//! Like `codegen_tests` in codegen.rs, but with no_std.

#![no_std]
#![allow(unused_macros)]

extern crate alloc;

mod codegen_tests {
    macro_rules! codegen_test {
        ($id:ident $name:tt $test:tt) => {
            mod $id {
                wit_bindgen_guest_rust::generate!({
                    path: $test,
                    world: $name,
                    no_std,
                });

                #[test]
                fn works() {}

                mod unchecked {
                    wit_bindgen_guest_rust::generate!({
                        path: $test,
                        world: $name,
                        unchecked,
                        no_std,
                    });

                    #[test]
                    fn works() {}
                }
            }

        };
    }
    test_helpers::codegen_tests!("*.wit");
}

mod strings {
    use alloc::string::String;

    wit_bindgen_guest_rust::generate!({
        inline: "
            default world not-used-name {
                import cat: interface {
                    foo: func(x: string)
                    bar: func() -> string
                }
            }
        ",
        no_std,
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
    use alloc::vec::Vec;

    wit_bindgen_guest_rust::generate!({
        inline: "
            default world not-used-name {
                import cat: interface {
                    foo: func(x: string)
                    bar: func() -> string
                }
            }
        ",
        raw_strings,
        no_std,
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
    use alloc::{
        format,
        string::{String, ToString},
    };

    mod bindings {
        wit_bindgen_guest_rust::generate!({
            inline: "
                default world baz {
                    export exports1: interface {
                        foo: func(x: string)
                        bar: func() -> string
                    }
                }
            ",
            macro_call_prefix: "bindings::",
            no_std,
        });

        pub(crate) use export_baz;
    }

    struct Component;

    impl bindings::exports1::Exports1 for Component {
        fn foo(x: String) {
            let _ = format!("foo: {}", x);
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
    use alloc::{format, string::String};

    wit_bindgen_guest_rust::generate!({
        inline: "
            default world baz {
                export exports2: interface {
                    foo: func(x: string)
                }
            }
        ",
        export_macro_name: "jam",
        no_std,
    });

    struct Component;

    impl exports2::Exports2 for Component {
        fn foo(x: String) {
            let _ = format!("foo: {}", x);
        }
    }

    jam!(Component);
}

mod skip {
    wit_bindgen_guest_rust::generate!({
        inline: "
            default world baz {
                export exports: interface {
                    foo: func()
                    bar: func()
                }
            }
        ",
        skip: ["foo"],
        no_std,
    });

    struct Component;

    impl exports::Exports for Component {
        fn bar() {}
    }

    export_baz!(Component);
}
