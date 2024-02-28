wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_import",
});

use test::resource_borrow_import::test::{foo, Thing};

pub struct Test {}

export!(Test);

impl Guest for Test {
    fn test(v: u32) -> u32 {
        foo(&Thing::new(v + 1)) + 4
    }
}
