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
        inline: r#"
        package test:paths;

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
