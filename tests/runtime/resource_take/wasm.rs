wit_bindgen::generate!({
    path: "../../tests/runtime/resource_take",
    exports: {
        world: Test,
        "test:resource-take/test": Test,
        "test:resource-take/test/thing": MyThing,
    },
});

use exports::test::resource_take::test::{Guest, GuestThing};
use wit_bindgen::rt::Resource;

pub struct Test;

impl Guest for Test {
    fn test() {
        let text = "Jabberwocky";
        assert_eq!(
            text,
            &Resource::take(Resource::new(MyThing(text.to_string()))).0
        );
    }
}

pub struct MyThing(String);

impl GuestThing for MyThing {
    fn new(text: String) -> Self {
        Self(text)
    }
}
