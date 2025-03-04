#![allow(unused_macros)]
#![allow(dead_code, unused_variables)]

mod symbol_does_not_conflict {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo1 {
                foo: func();
            }

            interface foo2 {
                foo: func();
            }

            interface bar1 {
                bar: func() -> string;
            }

            interface bar2 {
                bar: func() -> string;
            }

            world foo {
                export foo1;
                export foo2;
                export bar1;
                export bar2;
            }
        ",
    });

    struct Component;

    impl exports::my::inline::foo1::Guest for Component {
        fn foo() {}
    }

    impl exports::my::inline::foo2::Guest for Component {
        fn foo() {}
    }

    impl exports::my::inline::bar1::Guest for Component {
        fn bar() -> String {
            String::new()
        }
    }

    impl exports::my::inline::bar2::Guest for Component {
        fn bar() -> String {
            String::new()
        }
    }

    export!(Component);
}

mod alternative_bitflags_path {
    wit_bindgen::generate!({
        inline: "
            package my:inline;
            world foo {
                flags bar {
                    foo,
                    bar,
                    baz
                }
                export get-flag: func() -> bar;
            }
        ",
        bitflags_path: "my_bitflags",
    });

    pub(crate) use wit_bindgen::bitflags as my_bitflags;

    struct Component;

    export!(Component);

    impl Guest for Component {
        fn get_flag() -> Bar {
            Bar::BAZ
        }
    }
}

mod owned_resource_deref_mut {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                resource bar {
                    constructor(data: u32);
                    get-data: func() -> u32;
                    consume: static func(%self: bar) -> u32;
                }
            }

            world baz {
                export foo;
            }
        ",
    });

    pub struct MyResource {
        data: u32,
    }

    impl exports::my::inline::foo::GuestBar for MyResource {
        fn new(data: u32) -> Self {
            Self { data }
        }

        fn get_data(&self) -> u32 {
            self.data
        }

        fn consume(mut this: exports::my::inline::foo::Bar) -> u32 {
            let me: &MyResource = this.get();
            let prior_data: &u32 = &me.data;
            let new_data = prior_data + 1;
            let me: &mut MyResource = this.get_mut();
            let mutable_data: &mut u32 = &mut me.data;
            *mutable_data = new_data;
            me.data
        }
    }

    struct Component;

    impl exports::my::inline::foo::Guest for Component {
        type Bar = MyResource;
    }

    export!(Component);
}

mod package_with_versions {
    wit_bindgen::generate!({
        inline: "
            package my:inline@0.0.0;

            interface foo {
                resource bar {
                    constructor();
                }
            }

            world baz {
                export foo;
            }
        ",
    });

    pub struct MyResource;

    impl exports::my::inline::foo::GuestBar for MyResource {
        fn new() -> Self {
            loop {}
        }
    }

    struct Component;

    impl exports::my::inline::foo::Guest for Component {
        type Bar = MyResource;
    }

    export!(Component);
}

mod custom_derives {
    use std::collections::{hash_map::RandomState, HashSet};

    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface blah {
                record foo {
                    field1: string,
                    field2: list<u32>
                }

                bar: func(cool: foo);
            }

            world baz {
                export blah;
            }
        ",

        // Clone is included by default almost everywhere, so include it here to make sure it
        // doesn't conflict
        additional_derives: [serde::Serialize, serde::Deserialize, Hash, Clone, PartialEq, Eq],
    });

    use exports::my::inline::blah::Foo;

    struct Component;

    impl exports::my::inline::blah::Guest for Component {
        fn bar(cool: Foo) {
            // Check that built in derives that I've added actually work by seeing that this hashes
            let _blah: HashSet<Foo, RandomState> = HashSet::from_iter([Foo {
                field1: "hello".to_string(),
                field2: vec![1, 2, 3],
            }]);

            // Check that the attributes from an external crate actually work. If they don't work,
            // compilation will fail here
            let _ = serde_json::to_string(&cool);
        }
    }

    export!(Component);
}

mod with {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                record msg {
                    field: string,
                }
            }

            interface bar {
                use foo.{msg};

                bar: func(m: msg);
            }

            world baz {
                import bar;
            }
        ",
        with: {
            "my:inline/foo": other::my::inline::foo,
        },
    });

    pub mod other {
        wit_bindgen::generate!({
            inline: "
                package my:inline;

                interface foo {
                    record msg {
                        field: string,
                    }
                }

                world dummy {
                    use foo.{msg};
                    import bar: func(m: msg);
                }
            ",
        });
    }

    #[allow(dead_code)]
    fn test() {
        let msg = other::my::inline::foo::Msg {
            field: "hello".to_string(),
        };
        my::inline::bar::bar(&msg);
    }
}

mod with_and_resources {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                resource a;
            }

            interface bar {
                use foo.{a};

                bar: func(m: a) -> list<a>;
            }

            world baz {
                import bar;
            }
        ",
        with: {
            "my:inline/foo": other::my::inline::foo,
        },
    });

    pub mod other {
        wit_bindgen::generate!({
            inline: "
                package my:inline;

                interface foo {
                    resource a;
                }

                world dummy {
                    use foo.{a};
                    import bar: func(m: a);
                }
            ",
        });
    }
}

#[allow(unused)]
mod generate_unused_types {
    use exports::foo::bar::component::UnusedEnum;
    use exports::foo::bar::component::UnusedRecord;
    use exports::foo::bar::component::UnusedVariant;

    wit_bindgen::generate!({
        inline: "
            package foo:bar;

            world bindings {
                export component;
            }

            interface component {
                variant unused-variant {
                    %enum(unused-enum),
                    %record(unused-record)
                }
                enum unused-enum {
                    unused
                }
                record unused-record {
                    x: u32
                }
            }
        ",
        generate_unused_types: true,
    });
}

#[allow(unused)]
mod gated_features {
    wit_bindgen::generate!({
        inline: r#"
            package foo:bar@1.2.3;

            world bindings {
                @unstable(feature = x)
                import x: func();
                @unstable(feature = y)
                import y: func();
                @since(version = 1.2.3)
                import z: func();
            }
        "#,
        features: ["y"],
    });

    fn _foo() {
        y();
        z();
    }
}

#[allow(unused)]
mod simple_with_option {
    mod a {
        wit_bindgen::generate!({
            inline: r#"
                package foo:bar;

                interface a {
                    x: func();
                }

                package foo:baz {
                    world w {
                        import foo:bar/a;
                    }
                }
            "#,
            world: "foo:baz/w",
            generate_all,
        });
    }

    mod b {
        wit_bindgen::generate!({
            inline: r#"
                package foo:bar;

                interface a {
                    x: func();
                }

                package foo:baz {
                    world w {
                        import foo:bar/a;
                    }
                }
            "#,
            world: "foo:baz/w",
            with: { "foo:bar/a": generate },
        });
    }
}

#[allow(unused)]
mod multiple_paths {
    wit_bindgen::generate!({
        inline: r#"
        package test:paths;

        world test {
            import paths:path1/test;
            export paths:path2/test;
        }
        "#,
        path: ["tests/wit/path1", "tests/wit/path2"],
        generate_all,
    });
}

#[allow(unused)]
mod generate_custom_section_link_helpers {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world test {
                import a: interface {
                    x: func();
                }
            }
        "#,
        disable_custom_section_link_helpers: true,
    });
}

mod with_type {

    mod my_types {
        #[derive(Debug, Clone, Copy)]
        pub struct MyA {
            pub inner: f64,
        }

        #[derive(Debug, Clone, Copy)]
        pub struct MyB;

        impl MyB {
            pub fn take_handle(&self) -> u32 {
                0
            }

            pub fn from_handle(handle: u32) -> Self {
                Self
            }
        }

        pub enum MyC {
            A(MyA),
            B(MyB),
        }

        pub struct MyD {
            pub inner: u32,
        }

        pub struct MyE {
            pub inner: u32,
        }
    }

    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {

                record a {
                    inner: f64,
                }

                resource b;

                variant c {
                    a(a),
                    b(b),
                }

                // test type definition generation with `generate` option
                record f {
                    inner: u32,
                }

                // test type definition generation without `generate` option
                record g {
                    inner: u32,
                }

                func1: func(v: a) -> a;
                func2: func(v: b) -> b;
                func3: func(v: list<a>) -> list<b>;
                func4: func(v: option<a>) -> option<a>;
                func5: func() -> result<a>;
                func6: func() -> result<f>;
                func7: func() -> result<g>;
            }

            interface bar {
                record e {
                    inner: u32,
                }

                func6: func(v: e) -> e;
            }

            world dummy-type-remap {
                // test remapping with importing type directly
                use foo.{c};
                import func7: func(v: c) -> c;

                // test remapping the type defined in world
                record d {
                    inner: u32,
                }

                import func8: func(v: d) -> d;

                export bar;
            }
        ",
        with: {
            "my:inline/foo/a": crate::with_type::my_types::MyA,
            "my:inline/foo/b": crate::with_type::my_types::MyB,
            "my:inline/foo/c": crate::with_type::my_types::MyC,
            "dummy-type-remap/d": crate::with_type::my_types::MyD,
            "my:inline/bar/e": crate::with_type::my_types::MyE,
            "my:inline/foo/f": generate,
        },
    });

    pub struct Guest;

    impl exports::my::inline::bar::Guest for Guest {
        fn func6(v: my_types::MyE) -> my_types::MyE {
            v
        }
    }

    fn test() {
        let a = my_types::MyA { inner: 0.0 };
        let _ = my::inline::foo::func1(a);

        let b = my_types::MyB;
        let _ = my::inline::foo::func2(b);

        let c = my_types::MyC::A(a);
        let _ = func7(c);

        let a_list = vec![a, a];
        let _ = my::inline::foo::func3(&a_list);

        let _ = my::inline::foo::func4(Some(a));

        let _ = my::inline::foo::func5();

        let d = my_types::MyD { inner: 0 };
        let _ = func8(d);
    }
}
