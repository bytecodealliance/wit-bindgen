wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_simple",
});

pub struct Test {}

export!(Test);

impl Guest for Test {
    fn test_imports() {
        let r = R::new();
        test(&r);
    }
}
