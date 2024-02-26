wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_simple",
});

pub struct Test {}

export_resource_borrow_simple!(Test);

impl Guest for Test {
    fn test_imports() {
        let r = R::new();
        test(&r);
    }
}
