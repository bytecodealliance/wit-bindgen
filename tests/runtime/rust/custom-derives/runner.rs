include!(env!("BINDINGS"));

use crate::my::inline::blah::{bar, Foo};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        bar(&Foo {
            field1: "x".to_string(),
            field2: vec![2, 3, 3, 4],
        });
    }
}
