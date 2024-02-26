wit_bindgen::generate!({
    path: "../../tests/runtime/resource_into_inner",
    exports: {
        world: Test,
        "test:resource-into-inner/test": Test,
        "test:resource-into-inner/test/thing": MyThing,
    },
});

use exports::test::resource_into_inner::test::{Guest, GuestThing};

pub struct Test;

impl Guest for Test {
    fn test() {
        let text = "Jabberwocky";
        assert_eq!(
            text,
            &Resource::into_inner(Resource::new(MyThing(text.to_string()))).0
        );
    }
}

pub struct MyThing(String);

impl GuestThing for MyThing {
    fn new(text: String) -> Self {
        Self(text)
    }
}
