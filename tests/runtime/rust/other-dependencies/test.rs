wit_bindgen::generate!({
    inline: r#"
        package test:deps;

        world test {
            export other:test/test;
        }
    "#,
    path: "./other.wit",
    with: {
        "other:test/test": generate,
    }
});

use crate::exports::other::test::test::Guest;

struct Component;

export!(Component);

impl Guest for Component {
    fn f() {}
}
