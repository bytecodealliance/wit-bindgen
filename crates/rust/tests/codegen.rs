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
    use std::ops::Deref;

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

        interface ntl-bytes {
            type newtyped-bytes-list = list<u8>;
            type typed-bytes-list = list<u8>;

            use-newtyped-bytes-list: func(nl: newtyped-bytes-list) -> newtyped-bytes-list;
            use-typed-bytes-list: func(tl: typed-bytes-list) -> typed-bytes-list;
            use-bytes-list: func(l: list<u8>) -> list<u8>;
        }

        interface ntl-noncopy {
            // this will be new-typed by a non-copy struct
            type noncopy-byte = u8;

            type newtyped-noncopy-list = list<noncopy-byte>;
            type typed-noncopy-list = list<noncopy-byte>;

            use-newtyped-noncopy-list: func(nl: newtyped-noncopy-list) -> newtyped-noncopy-list;
            use-typed-noncopy-list: func(tl: typed-noncopy-list) -> typed-noncopy-list;
            use-noncopy-list: func(l: list<noncopy-byte>) -> list<noncopy-byte>;
        }

        interface ntl-noncanonical {
            // tuples are non-canonical, but can implement copy
            type noncanonical = tuple<u8,u8>;

            type newtyped-noncanonical-list = list<noncanonical>;
            type typed-noncanonical-list = list<noncanonical>;

            use-newtyped-noncanonical-list: func(nl: newtyped-noncanonical-list) -> newtyped-noncanonical-list;
            use-typed-noncanonical-list: func(tl: typed-noncanonical-list) -> typed-noncanonical-list;
            use-noncanonical-list: func(l: list<noncanonical>) -> list<noncanonical>;
        }

        world test {
            import ntl;
            export ntl;
            import ntl-bytes;
            export ntl-bytes;
            import ntl-noncopy;
            export ntl-noncopy;
            import ntl-noncanonical;
            export ntl-noncanonical;
        }
        "#,
        with: {
            "test:newtyped-list/ntl/newtyped-list": crate::newtyped_list::NewtypedList,
            "test:newtyped-list/ntl-bytes/newtyped-bytes-list": bytes::Bytes,
            "test:newtyped-list/ntl-noncopy/noncopy-byte": crate::newtyped_list::NoncopyByte,
            "test:newtyped-list/ntl-noncopy/newtyped-noncopy-list": crate::newtyped_list::NewtypedNoncopyList,
            "test:newtyped-list/ntl-noncanonical/newtyped-noncanonical-list": crate::newtyped_list::NewtypedNoncanonicalList,
        }
    });

    pub struct NewtypedList(Vec<u8>);

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

    pub struct NoncopyByte(u8);
    pub struct NewtypedNoncopyList(Vec<String>);

    impl From<Vec<String>> for NewtypedNoncopyList {
        fn from(value: Vec<String>) -> Self {
            NewtypedNoncopyList(value)
        }
    }

    impl From<NewtypedNoncopyList> for Vec<String> {
        fn from(value: NewtypedNoncopyList) -> Self {
            value.0
        }
    }

    impl Deref for NewtypedNoncopyList {
        type Target = Vec<String>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    pub struct NewtypedNoncanonicalList(Vec<(u8, u8)>);

    impl From<Vec<(u8, u8)>> for NewtypedNoncanonicalList {
        fn from(value: Vec<(u8, u8)>) -> Self {
            NewtypedNoncanonicalList(value)
        }
    }

    impl From<NewtypedNoncanonicalList> for Vec<(u8, u8)> {
        fn from(value: NewtypedNoncanonicalList) -> Self {
            value.0
        }
    }

    impl Deref for NewtypedNoncanonicalList {
        type Target = Vec<(u8, u8)>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
