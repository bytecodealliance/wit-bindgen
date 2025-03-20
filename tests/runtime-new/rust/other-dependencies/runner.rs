wit_bindgen::generate!({
    inline: r#"
        package test:deps;

        world test {
            import other:test/test;
        }
    "#,
    path: "./other.wit",
    with: {
        "other:test/test": generate,
    }
});

fn main() {
    other::test::test::f();
}
