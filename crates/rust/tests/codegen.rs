#![allow(unused_macros, reason = "testing codegen, not functionality")]
#![allow(
    dead_code,
    unused_variables,
    reason = "testing codegen, not functionality"
)]

#[allow(unused, reason = "testing codegen, not functionality")]
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

#[allow(unused, reason = "testing codegen, not functionality")]
mod inline_and_path {
    wit_bindgen::generate!({
        // A different package from `multiple_paths` above. Two different
        // worlds under one name corrupt each other's `component-type` sections
        // on wasm, and collide on the world marker symbol natively. So this
        // technically is a fix to the existing tests
        inline: r#"
        package test:inline-and-path-root;

        world test {
            import test:inline-and-path/bar;
        }
        "#,
        path: ["tests/wit/path3"],
        generate_all,
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod newtyped_list {
    use std::ops::Deref;

    wit_bindgen::generate!({
        inline: r#"
        package test:newtyped-list;

        interface byte {
            type typed-list-of-byte = list<u8>;
            type newtyped-list-of-byte = list<u8>;

            record rec-of-lists {
                l: list<u8>,
                tl: typed-list-of-byte,
                nl: newtyped-list-of-byte,
            }

            use-list-of-byte: func(l: list<u8>) -> list<u8>;
            use-typed-list-of-byte: func(tl: typed-list-of-byte) -> typed-list-of-byte;
            use-newtyped-list-of-byte: func(nl: newtyped-list-of-byte) -> newtyped-list-of-byte;
            use-rec-of-lists: func(t: rec-of-lists) -> rec-of-lists;
        }

        interface noncopy-byte {
            // this will be new-typed by a non-copy struct
            type noncopy-byte = u8;

            type newtyped-list-of-noncopy-byte = list<noncopy-byte>;
            type typed-list-of-noncopy-byte = list<noncopy-byte>;

            record rec-of-lists-of-noncopy-byte {
                ntl: newtyped-list-of-noncopy-byte,
                tl: typed-list-of-noncopy-byte,
                l: list<noncopy-byte>,
            }

            use-newtyped-list-of-noncopy-byte: func(nl: newtyped-list-of-noncopy-byte) -> newtyped-list-of-noncopy-byte;
            use-typed-list-of-noncopy-byte: func(tl: typed-list-of-noncopy-byte) -> typed-list-of-noncopy-byte;
            use-list-of-noncopy-byte: func(l: list<noncopy-byte>) -> list<noncopy-byte>;
            use-rec-of-lists-of-noncopy-byte: func(t: rec-of-lists-of-noncopy-byte) -> rec-of-lists-of-noncopy-byte;
        }

        world test {
            import byte;
            export byte;
            import noncopy-byte;
            export noncopy-byte;
        }
        "#,
        with: {
            "test:newtyped-list/byte/newtyped-list-of-byte": crate::newtyped_list::NewtypedListOfByte,
            "test:newtyped-list/noncopy-byte/noncopy-byte": crate::newtyped_list::NoncopyByte,
            "test:newtyped-list/noncopy-byte/newtyped-list-of-noncopy-byte": crate::newtyped_list::NewtypedListofNoncopyByte,
        }
    });

    #[derive(Debug, Clone)]
    pub struct NewtypedListOfByte(Vec<u8>);

    impl From<Vec<u8>> for NewtypedListOfByte {
        fn from(value: Vec<u8>) -> Self {
            NewtypedListOfByte(value)
        }
    }

    impl From<NewtypedListOfByte> for Vec<u8> {
        fn from(value: NewtypedListOfByte) -> Self {
            value.0
        }
    }

    impl Deref for NewtypedListOfByte {
        type Target = Vec<u8>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(Debug, Clone)]
    pub struct NoncopyByte(u8);

    #[derive(Debug, Clone)]
    pub struct NewtypedListofNoncopyByte(Vec<NoncopyByte>);

    impl From<Vec<NoncopyByte>> for NewtypedListofNoncopyByte {
        fn from(value: Vec<NoncopyByte>) -> Self {
            NewtypedListofNoncopyByte(value)
        }
    }

    impl From<NewtypedListofNoncopyByte> for Vec<NoncopyByte> {
        fn from(value: NewtypedListofNoncopyByte) -> Self {
            value.0
        }
    }

    impl Deref for NewtypedListofNoncopyByte {
        type Target = Vec<NoncopyByte>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod map_type_hashmap {
    wit_bindgen::generate!({
        inline: r#"
        package test:map-type;

        interface maps {
            type names-by-id = map<u32, string>;

            roundtrip: func(a: names-by-id) -> names-by-id;
            inline-roundtrip: func(a: map<string, u32>) -> map<string, u32>;
        }

        world test {
            import maps;
            export maps;
        }
        "#,
        map_type: "std::collections::HashMap",
        generate_all,
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod map_type_default {
    wit_bindgen::generate!({
        inline: r#"
        package test:map-default;

        interface maps {
            type names-by-id = map<u32, string>;
            nested: func(a: map<string, map<u32, string>>) -> map<string, map<u32, string>>;
        }

        world test {
            import maps;
            export maps;
        }
        "#,
        generate_all,
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod retyped_list {
    use std::ops::Deref;

    wit_bindgen::generate!({
        inline: r#"
        package test:retyped-list;

        interface bytes {
            // this will be the `Bytes` type from the bytes crate
            type retyped-list-as-bytes = list<u8>;

            record rec-bytes {
                rl: retyped-list-as-bytes,
            }

            use-retyped-list-as-bytes: func(ri: retyped-list-as-bytes) -> retyped-list-as-bytes;
            use-rec-of-retyped-list-as-bytes: func(rl: retyped-list-as-bytes) -> retyped-list-as-bytes;
        }

        world test {
            import bytes;
            export bytes;
        }
        "#,
        with: {
            "test:retyped-list/bytes/retyped-list-as-bytes": bytes::Bytes,
        }
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod owning_method_chaining {
    wit_bindgen::generate!({
        inline: r#"
        package test:method-chaining;
        world test {
            resource a {
                constructor();
                set-a: func(arg: u32);
                set-b: func(arg: bool);
                do: func();
            }
        }
        "#,
        generate_all,
        chainable_methods: ["all"]
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod borrowing_method_chaining {
    wit_bindgen::generate!({
        inline: r#"
        package test:borrowing-method-chaining;
        world test {
            resource a {
                constructor();
                set-a: func(arg: u32);
                set-b: func(arg: bool);
                do: func();
            }
        }
        "#,
        generate_all,
        chainable_methods: ["&all"]
    });
}

#[allow(unused, reason = "testing codegen, not functionality")]
mod merge_structurally_equal_types {
    wit_bindgen::generate!({
        inline: r#"
        package test:merge-structurally-equal-types;

        interface blag {
            variant kind1 { a, b(u64), c }
            variant kind2 { a, b(u64), c }
            record kind3 { a: input-stream }
            record kind4 { a: input-stream }
            record tree { l: t1, r: t1  }
            record t1 { l: t2, r: t2 }
            record t2 { l: t3, r: t3 }
            record t3 { l: kind1, r: kind2 }
            record t-stream  { tree: tree, %stream: option<borrow<input-stream>> }
            resource input-stream {
                read: func(len: u64) -> list<u8>;
            }
            f: func(x: kind1) -> kind2;
            g: func(x: kind3) -> kind4;
            h: func(x: t-stream) -> tree;
        }

        interface blah {
            use blag.{input-stream, kind4, t-stream};
            variant kind5 { a, b(u64), c }
            variant kind6 { a, c, b(u64) }
            record kind7 { a: borrow<input-stream> }
            record tt { l: t2, r: t2  }
            record t1 { l: t3, r: t3 }
            record t2 { l: t1, r: t1 }
            record t3 { l: kind5, r: kind5 }
            variant custom-result { ok(tt), err }
            f: func(x: kind6) -> kind5;
            g: func(x: kind7) -> kind4;
            h: func(x: t-stream) -> custom-result;

            record r1 { a: u8 }
            type a1 = u8;
            record r2 { a: a1 }
            alias-type: func(x: r1) -> r2;
        }

        interface resources {
            resource r1;
            type r2 = r1;

            record t1 { a: r1 }
            record t2 { a: r2 }
            alias-own: func(x: t1) -> t2;
            alias-aggregate: func(x: option<t1>) -> option<t2>;
        }

        world proxy {
            import blag;
            export blag;
            import blah;
            export blah;
        }
        "#,
        generate_all,
        merge_structurally_equal_types: true
    });
}

mod targeted_attributes {
    wit_bindgen::generate!({
        inline: r#"
        package test:attrs;

        interface iface {
            record point { x: u32, y: u32 }
            enum color { red, green, blue }
            variant shape { circle(u32), square(point), a-b }

            // used as both return and param, so `duplicate_if_necessary` generates
            // an owned `String` form and a borrowed `&str` form
            record label { text: string }

            paint: func(c: color) -> color;
            draw: func(s: shape) -> shape;
            round: func(p: point) -> point;
            make-label: func() -> label;
            use-label: func(l: label) -> u32;
        }

        world test {
            import iface;
        }
        "#,
        generate_all,
        ownership: Borrowing { duplicate_if_necessary: true },
        additional_type_attributes: {
            // two entries for one selector, both apply
            "test:attrs/iface/point": [#[derive(PartialEq, Eq)] #[derive(Hash)]],
            "test:attrs/iface/color": [#[doc = "a primary color"] #[derive(Hash)]],
            "test:attrs/iface/shape": [#[derive(PartialEq)]],
            "test:attrs/iface/label": [#[derive(PartialEq, Eq)]],
        },
        additional_member_attributes: {
            "test:attrs/iface/point.x": [#[allow(dead_code)]],
            "test:attrs/iface/color.red": [#[doc = "the primary color"]],
            "test:attrs/iface/shape.circle": [#[doc = "a round shape"]],
            // `a-b` upper-camels to `AB`, so this matches only via the raw wit name
            "test:attrs/iface/shape.a-b": [#[allow(dead_code)]],
            "test:attrs/iface/label.text": [#[allow(dead_code)]],
        },
    });

    #[test]
    fn injected_derives_are_usable() {
        use test::attrs::iface::{Color, LabelParam, LabelResult, Point, Shape};

        let a = Point { x: 1, y: 2 };
        assert_eq!(a, a.clone());
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&Point { x: 1, y: 2 }));

        let mut colors = std::collections::HashSet::new();
        colors.insert(Color::Red);
        assert!(colors.contains(&Color::Red));
        assert!(!colors.contains(&Color::Blue));

        assert_eq!(Shape::Circle(3), Shape::Circle(3).clone());
        assert_ne!(Shape::Circle(3), Shape::Circle(4));

        // both the owned `String` form and the borrowed `&str` form got the derives
        assert_eq!(
            LabelResult { text: "hi".into() },
            LabelResult { text: "hi".into() }
        );
        assert_eq!(LabelParam { text: "hi" }, LabelParam { text: "hi" });
        assert_ne!(LabelParam { text: "hi" }, LabelParam { text: "bye" });
    }
}

// Guards where `@version` lands in a qualified selector: after the interface,
// before the type.
mod versioned_selectors {
    wit_bindgen::generate!({
        inline: r#"
        package test:hier@1.2.3;
        interface types {
            record alpha { x: u32 }
            get-alpha: func() -> alpha;
        }
        world w { import types; }
        "#,
        generate_all,
        additional_type_attributes: {
            "test:hier/types@1.2.3/alpha": [#[derive(PartialEq, Eq)] #[derive(PartialOrd, Ord)]],
        },
        additional_member_attributes: {
            "test:hier/types@1.2.3/alpha.x": [#[allow(dead_code)]],
        },
    });

    #[test]
    fn versioned_selectors_resolve() {
        use test::hier::types::Alpha;

        assert_eq!(Alpha { x: 1 }, Alpha { x: 1 });
        assert!(Alpha { x: 1 } < Alpha { x: 2 });
    }
}

mod accessors {
    wit_bindgen::generate!({
        inline: r#"
        package test:accessors;

        interface i {
            prop: get() -> u64;
            prop: set(value: u64);

            read-only: get() -> string;

            fallible: get() -> u32;
            fallible: set(value: u32) -> result<_, string>;

            %type: get() -> u32;
            %type: set(value: u32);

            get: func();
            set: func(value: u32);

            resource blob {
                constructor();
                position: get() -> u64;
                position: set(value: u64);
                label: get() -> string;
                label: set(value: string) -> result<_, string>;
                max-size: static get() -> u64;
                max-size: static set(value: u64) -> result<_, string>;
            }
        }

        world test {
            import i;
            export i;

            import world-prop: get() -> u64;
            import world-prop: set(value: u64);
            export other-prop: get() -> string;
            export other-prop: set(value: string) -> result<_, string>;
        }
        "#,
        generate_all,
    });

    fn use_imports() {
        use test::accessors::i::*;

        let _: u64 = prop();
        let _: () = set_prop(1);
        let _: String = read_only();
        let _: u32 = fallible();
        let _: Result<(), String> = set_fallible(1);
        let _: u32 = type_();
        let _: () = set_type(1);
        let _: () = get();
        let _: () = set(1);

        let b = Blob::new();
        let _: u64 = b.position();
        let _: () = b.set_position(1);
        let _: String = b.label();
        let _: Result<(), String> = b.set_label("x");
        let _: u64 = Blob::max_size();
        let _: Result<(), String> = Blob::set_max_size(1);

        let _: u64 = world_prop();
        let _: () = set_world_prop(1);
    }

    struct Component;

    impl Guest for Component {
        fn other_prop() -> String {
            todo!()
        }
        fn set_other_prop(value: String) -> Result<(), String> {
            todo!()
        }
    }

    impl exports::test::accessors::i::Guest for Component {
        type Blob = MyBlob;

        fn prop() -> u64 {
            todo!()
        }
        fn set_prop(value: u64) {
            todo!()
        }
        fn read_only() -> String {
            todo!()
        }
        fn fallible() -> u32 {
            todo!()
        }
        fn set_fallible(value: u32) -> Result<(), String> {
            todo!()
        }
        fn type_() -> u32 {
            todo!()
        }
        fn set_type(value: u32) {
            todo!()
        }
        fn get() {
            todo!()
        }
        fn set(value: u32) {
            todo!()
        }
    }

    struct MyBlob;

    impl exports::test::accessors::i::GuestBlob for MyBlob {
        fn new() -> MyBlob {
            todo!()
        }
        fn position(&self) -> u64 {
            todo!()
        }
        fn set_position(&self, value: u64) {
            todo!()
        }
        fn label(&self) -> String {
            todo!()
        }
        fn set_label(&self, value: String) -> Result<(), String> {
            todo!()
        }
        fn max_size() -> u64 {
            todo!()
        }
        fn set_max_size(value: u64) -> Result<(), String> {
            todo!()
        }
    }
}

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
