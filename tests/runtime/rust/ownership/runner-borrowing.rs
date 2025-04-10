//@ args = '--ownership borrowing'

include!(env!("BINDINGS"));

impl PartialEq for thing_in_and_out::Thing {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

fn main() {
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

    let strings = vec!["foo", "bar", "baz"];
    let resource = test::ownership::both_list_and_resource::TheResource::new(&strings);
    test::ownership::both_list_and_resource::list_and_resource(
        test::ownership::both_list_and_resource::Thing {
            a: strings.iter().map(|s| s.to_string()).collect(),
            b: resource,
        },
    );
}
