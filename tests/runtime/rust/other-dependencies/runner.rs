wit_bindgen::generate!({
    inline: r#"
        package test:deps;

        world test {
            import other:test/test;

            export run: func();
        }
    "#,
    path: "./other.wit",
    with: {
        "other:test/test": generate,
    }
});

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        other::test::test::f();
    }
}
