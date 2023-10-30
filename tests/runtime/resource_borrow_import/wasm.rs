wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_import",
    exports: {
        world: Test,
        "test:resource-borrow-import/test": Test,
    },
});

use test::resource_borrow_import::test::{foo, Thing};

pub struct Test {}

impl Guest for Test {
    fn test(v: u32,) -> u32{ 
        foo(&Thing::new(v + 1)) + 4
    }
}
