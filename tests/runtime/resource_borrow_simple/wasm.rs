wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_simple",
    exports: {
        world: Test,
    }
});

pub struct Test {}

impl Guest for Test {
    fn test_imports() {
        unsafe {
            let r = R { handle: wit_bindgen::rt::Resource::from_handle(0) };
            test(&r);
        }
    }
}