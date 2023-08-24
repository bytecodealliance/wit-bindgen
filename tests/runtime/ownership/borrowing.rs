wit_bindgen::generate!({
    path: "../../tests/runtime/ownership",
    exports: {
        world: Exports
    },
    ownership: Borrowing {
        duplicate_if_necessary: false
    }
});

impl PartialEq for thing_in_and_out::Thing {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

struct Exports;

impl Guest for Exports {
    fn foo() {
        let value = &[&["foo", "bar"] as &[_]] as &[_];
        assert_eq!(
            vec![vec!["foo".to_owned(), "bar".to_owned()]],
            lists::foo(value)
        );

        thing_in::bar(thing_in::Thing {
            name: "thing 1",
            value: &["some value", "another value"],
        });

        let value = thing_in_and_out::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        };
        assert_eq!(value, thing_in_and_out::baz(&value));
    }
}
