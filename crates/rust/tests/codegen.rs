#![allow(unused_macros)]
#![allow(dead_code, unused_variables)]

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
