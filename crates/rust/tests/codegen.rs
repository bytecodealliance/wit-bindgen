#![allow(unused_macros)]
#![allow(dead_code, unused_variables)]

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

#[allow(unused)]
mod newtyped_list {
    wit_bindgen::generate!({
        inline: r#"
        package test:newtyped-list;

        interface ntl {
            type newtyped-list = list<u8>;
            type typed-list = list<u8>;

            use-newtyped-list: func(nl: newtyped-list) -> newtyped-list;
            use-typed-list: func(tl: typed-list) -> typed-list;
            use-list: func(l: list<u8>) -> list<u8>;
        }

        world test {
            import ntl;
            export ntl;
        }
        "#,
        with: {
            "test:newtyped-list/ntl/newtyped-list": crate::newtyped_list::NewtypedList,
        }
    });

    struct NewtypedList(Vec<u8>);

    impl From<Vec<u8>> for NewtypedList {
        fn from(value: Vec<u8>) -> Self {
            NewtypedList(value)
        }
    }

    impl From<NewtypedList> for Vec<u8> {
        fn from(value: NewtypedList) -> Self {
            value.0
        }
    }
}
