wit_bindgen::generate!({
    inline: r#"
    package test:deps;

    world test {
        export other:test/test;
    }
    "#,
    path: "../../tests/runtime/other-dependencies/other",
});
