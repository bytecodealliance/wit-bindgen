wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_simple",
    exports: {
        world: Test,
    }
});

pub struct Test {}

impl Guest for Test {
    fn test_imports() {
        let r = R::new();
        test(&r);
    }
}