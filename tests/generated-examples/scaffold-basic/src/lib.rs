// Generated component implementation for world 'basic-test'
// TODO: Implement the functions marked with TODO comments

wit_bindgen::generate!({
    world: "basic-test",
    path: "/tmp/test-basic",
    // Uncomment to see generated module paths:
    // show_module_paths: true,
});

struct Component;

impl exports::test::Guest for Component {
    fn hello() -> /* TODO: Add return type */ {
        // TODO: Implement hello
        todo!()
    }

}

export!(Component);
