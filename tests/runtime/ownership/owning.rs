wit_bindgen::generate!({
    path: "../../tests/runtime/ownership",
    exports: {
        world: Exports
    },
    ownership: Owning
});

impl PartialEq for thing_in_and_out::Thing {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

struct Exports;

impl Ownership for Exports {
    fn foo() {
        let value = vec![vec!["foo".to_owned(), "bar".to_owned()]];
        assert_eq!(value, lists::foo(&value));

        thing_in::bar(&thing_in::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        });

        let value = thing_in_and_out::Thing {
            name: "thing 1".to_owned(),
            value: vec!["some value".to_owned(), "another value".to_owned()],
        };
        assert_eq!(value, thing_in_and_out::baz(&value));
    }
}
