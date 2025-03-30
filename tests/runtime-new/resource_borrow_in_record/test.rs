include!(env!("BINDINGS"));

use crate::exports::test::resource_borrow_in_record::to_test::{Foo, Guest, GuestThing, Thing};

export!(Component);

struct Component;

impl Guest for Component {
    type Thing = MyThing;

    fn test(list: Vec<Foo<'_>>) -> Vec<Thing> {
        list.iter()
            .map(|foo| {
                Thing::new(MyThing {
                    contents: format!("{} test", foo.thing.get::<MyThing>().contents),
                })
            })
            .collect()
        // ..
    }
}

#[derive(Clone)]
struct MyThing {
    contents: String,
}

impl GuestThing for MyThing {
    fn new(msg: String) -> MyThing {
        MyThing {
            contents: format!("{msg} new"),
        }
    }
    fn get(&self) -> String {
        format!("{} get", self.contents)
    }
}
