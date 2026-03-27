include!(env!("BINDINGS"));

use exports::test::resource_into_inner::to_test::{Guest, GuestThing, Thing};

pub struct Test;

export!(Test);

impl Guest for Test {
    type Thing = MyThing;

    fn test() {
        let text = "Jabberwocky";
        let thing = Thing::new(MyThing(text.to_string()));
        let inner: MyThing = thing.into_inner();
        assert_eq!(text, &inner.0);
    }
}

pub struct MyThing(String);

impl GuestThing for MyThing {
    fn new(text: String) -> Self {
        Self(text)
    }
}
