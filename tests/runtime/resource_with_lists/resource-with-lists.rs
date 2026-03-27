include!(env!("BINDINGS"));

use exports::test::resource_with_lists::test::GuestThing;
use test::resource_with_lists::test::Thing;

pub struct Test {}

export!(Test);

impl exports::test::resource_with_lists::test::Guest for Test {
    type Thing = MyThing;
}

pub struct MyThing {
    val: Thing,
}

impl GuestThing for MyThing {
    fn new(l: Vec<u8>) -> Self {
        let mut result = l.clone();
        result.extend_from_slice(" Thing".as_bytes());
        let result = Thing::new(&result);
        Self { val: result }
    }
    fn foo(&self) -> Vec<u8> {
        let mut list = self.val.foo().clone();
        list.extend_from_slice(" Thing.foo".as_bytes());
        list
    }

    fn bar(&self, l: Vec<u8>) {
        let mut result = l.clone();
        result.extend_from_slice(" Thing.bar".as_bytes());
        self.val.bar(&result);
    }

    fn baz(l: Vec<u8>) -> Vec<u8> {
        let mut result = l.clone();
        result.extend_from_slice(" Thing.baz".as_bytes());
        let mut list2 = Thing::baz(&result).clone();
        list2.extend_from_slice(" Thing.baz again".as_bytes());
        list2
    }
}
