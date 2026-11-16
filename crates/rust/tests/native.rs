#![allow(unused_macros, reason = "testing codegen, not functionality")]
#![allow(
    dead_code,
    unused_variables,
    reason = "testing codegen, not functionality"
)]

// These call `export!` so the native export symbols, which live inside the
// `__export_*_cabi!` macro, actually get compiled.
#[allow(unused, reason = "testing codegen, not functionality")]
mod native_symbols {
    wit_bindgen::generate!({
        inline: r#"
        package test:native;

        interface operations {
            resource thing {
                constructor(x: u32);
                get: func() -> u32;
            }
            add: func(a: u32, b: u32) -> u32;
            describe: func(value: u32) -> string;
        }

        world test {
            import operations;
            export operations;
        }
        "#,
        generate_all,
    });

    struct Component;

    impl exports::test::native::operations::Guest for Component {
        type Thing = MyThing;

        fn add(a: u32, b: u32) -> u32 {
            a + b
        }

        fn describe(value: u32) -> String {
            value.to_string()
        }
    }

    struct MyThing(u32);

    impl exports::test::native::operations::GuestThing for MyThing {
        fn new(x: u32) -> Self {
            MyThing(x)
        }

        fn get(&self) -> u32 {
            self.0
        }
    }

    export!(Component);
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod native_symbols_async {
    wit_bindgen::generate!({
        inline: r#"
        package test:native-async;

        interface operations {
            describe: async func(value: u32) -> string;
            pipe: async func(input: future<u32>, bytes: stream<u8>) -> future<string>;
        }

        world test {
            import operations;
            export operations;
        }
        "#,
        generate_all,
    });

    struct Component;

    impl exports::test::native_async::operations::Guest for Component {
        async fn describe(value: u32) -> String {
            value.to_string()
        }

        async fn pipe(
            _input: wit_bindgen::FutureReader<u32>,
            _bytes: wit_bindgen::StreamReader<u8>,
        ) -> wit_bindgen::FutureReader<String> {
            todo!()
        }
    }

    export!(Component);
}

// Import shims define no symbols, so two worlds importing the same
// interface link side by side.
#[allow(unused, reason = "testing codegen, not functionality")]
mod native_symbols_shared_one {
    wit_bindgen::generate!({
        inline: r#"
        package test:native-shared;
        interface operations { add: func(a: u32, b: u32) -> u32; }
        world one { import operations; }
        "#,
        generate_all,
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod native_symbols_shared_two {
    wit_bindgen::generate!({
        inline: r#"
        package test:native-shared;
        interface operations { add: func(a: u32, b: u32) -> u32; }
        world two { import operations; }
        "#,
        generate_all,
    });
}

// Binding the *same* world twice needs `type_section_suffix` for the world
// marker and `export_prefix` for the export names; drop either and it fails
// to link with a duplicate symbol.
macro_rules! native_symbols_same_world {
    ($module:ident, $tag:literal) => {
        #[allow(unused, reason = "testing codegen, not functionality")]
        mod $module {
            wit_bindgen::generate!({
                inline: r#"
                package test:native-same-world;
                world w { export run: func() -> u32; }
                "#,
                generate_all,
                type_section_suffix: $tag,
                export_prefix: $tag,
            });

            struct Component;

            impl Guest for Component {
                fn run() -> u32 {
                    0
                }
            }

            export!(Component);
        }
    };
}

native_symbols_same_world!(native_symbols_same_world_a, "a_");
native_symbols_same_world!(native_symbols_same_world_b, "b_");

#[cfg(not(target_family = "wasm"))]
#[test]
fn native_world_metadata_decodes() {
    unsafe extern "C" {
        fn __wit_bindgen_world_testX3AnativeX2Ftest(len: *mut usize) -> *const u8;
    }

    let mut len = 0usize;
    let ptr = unsafe { __wit_bindgen_world_testX3AnativeX2Ftest(&mut len) };
    assert!(!ptr.is_null());
    assert!(len > 0);
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };

    let (resolve, pkg_id) = match wit_component::decode(bytes).unwrap() {
        wit_component::DecodedWasm::WitPackage(resolve, pkg_id) => (resolve, pkg_id),
        wit_component::DecodedWasm::Component(..) => panic!("expected an encoded WIT package"),
    };
    assert_eq!(resolve.packages[pkg_id].name.to_string(), "test:native");
    let world_id = resolve.select_world(&[pkg_id], None).unwrap();
    let world = &resolve.worlds[world_id];
    assert_eq!(world.name, "test");
    assert!(!world.imports.is_empty());
    assert!(!world.exports.is_empty());
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn native_import_resolver_dispatches() {
    unsafe extern "C" fn host_add(a: i32, b: i32) -> i32 {
        a + b
    }

    unsafe {
        wit_bindgen::rt::register_import_resolver(|module, name| {
            if module == "test:native/operations" && name == "add" {
                core::ptr::NonNull::new(host_add as *mut ())
            } else {
                None
            }
        });
    }

    assert_eq!(native_symbols::test::native::operations::add(20, 22), 42);
}

#[test]
fn native_future_vtable_uses_target_pointer_size() {
    use native_symbols_async::wit_future::FuturePayload;
    let layout = <String as FuturePayload>::VTABLE.layout;
    assert_eq!(layout.size(), 2 * core::mem::size_of::<*const u8>());
    assert_eq!(layout.align(), core::mem::align_of::<*const u8>());
}
