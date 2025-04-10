//@ args = '--ownership owning'

include!(env!("BINDINGS"));
impl PartialEq for thing_in_and_out::Thing {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value == other.value
    }
}

fn main() {
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

    let strings = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
    let resource = test::ownership::both_list_and_resource::TheResource::new(&strings);
    test::ownership::both_list_and_resource::list_and_resource(
        test::ownership::both_list_and_resource::Thing {
            a: strings,
            b: resource,
        },
    );
}
