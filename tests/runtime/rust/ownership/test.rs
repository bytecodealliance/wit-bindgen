include!(env!("BINDINGS"));

use crate::exports::test::ownership::both_list_and_resource;
use crate::exports::{lists, thing_in, thing_in_and_out};

struct Component;

export!(Component);

struct MyResource(Vec<String>);

impl lists::Guest for Component {
    fn foo(list: Vec<Vec<String>>) -> Vec<Vec<String>> {
        list
    }
}

impl thing_in::Guest for Component {
    fn bar(_value: thing_in::Thing) {}
}

impl thing_in_and_out::Guest for Component {
    fn baz(value: thing_in_and_out::Thing) -> thing_in_and_out::Thing {
        value
    }
}

impl both_list_and_resource::Guest for Component {
    type TheResource = MyResource;

    fn list_and_resource(value: both_list_and_resource::Thing) {
        assert_eq!(value.a, value.b.get::<MyResource>().0);
    }
}

impl both_list_and_resource::GuestTheResource for MyResource {
    fn new(list: Vec<String>) -> MyResource {
        MyResource(list)
    }
}
