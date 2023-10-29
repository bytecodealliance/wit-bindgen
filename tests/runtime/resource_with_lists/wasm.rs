wit_bindgen::generate!({
    path: "../../tests/runtime/resource_with_lists",
    exports: {
        world: Test,
        "test:resource-with-lists/test/thing": MyThing,
    }
});

use test::resource_with_lists::test::Thing;
use exports::test::resource_with_lists::test::GuestThing;

pub struct Test {}

pub struct MyThing {
    val: Thing,
}

impl GuestThing for MyThing {
    fn new(l: wit_bindgen::rt::vec::Vec::<u8>,) -> Self { 
        let mut result = l.clone();
        result.extend_from_slice(" Thing".as_bytes());
        let result = Thing::new(&result);
        Self {
            val: result,
        }
    }
    fn foo(&self,) -> wit_bindgen::rt::vec::Vec::<u8>{ 
        let mut list = self.val.foo().clone();
        list.extend_from_slice(" Thing.foo".as_bytes());
        list
    }

    fn bar(&self,l: wit_bindgen::rt::vec::Vec::<u8>,){ 
        let mut result = l.clone();
        result.extend_from_slice(" Thing.bar".as_bytes());
        self.val.bar(&result);
    }
  
    fn baz(l: wit_bindgen::rt::vec::Vec::<u8>,) -> wit_bindgen::rt::vec::Vec::<u8>{ 
        let mut result = l.clone();
        result.extend_from_slice(" Thing.baz".as_bytes());
        let mut list2 = Thing::baz(&result).clone();
        list2.extend_from_slice(" Thing.baz again".as_bytes());
        list2
    }
}