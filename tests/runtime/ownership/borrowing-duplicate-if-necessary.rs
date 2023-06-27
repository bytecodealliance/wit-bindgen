wit_bindgen::generate!({
    path: "../../tests/runtime/ownership",
    ownership: Borrowing {
        duplicate_if_necessary: true
    }
});

impl PartialEq for thing_in_and_out::ThingResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

struct Exports;

export_ownership!(Exports);

impl Ownership for Exports {
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

        let value = thing_in_and_out::ThingParam {
            name: "thing 1",
            value: &["some value", "another value"],
        };
        assert_eq!(
            thing_in_and_out::ThingResult {
                name: "thing 1".to_owned(),
                value: vec!["some value".to_owned(), "another value".to_owned()],
            },
            thing_in_and_out::baz(value)
        );
    }
}
