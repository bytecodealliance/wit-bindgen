//@ args = '--with my:inline/foo=other::my::inline::foo'

#![expect(
    unused_imports,
    reason = "using `with` is known to produce possibly dead imports"
)]

include!(env!("BINDINGS"));

mod other {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                record a { b: u8 }

                bar: func(a: a);
            }

            world gen {
                import foo;
            }
        ",
    });
}

struct Component;

export!(Component);

use crate::exports::my::inline::foo::{Guest, A};
use std::any::TypeId;

impl Guest for Component {
    fn bar(a: A) {
        assert!(TypeId::of::<A>() != TypeId::of::<other::my::inline::foo::A>());
        other::my::inline::foo::bar(other::my::inline::foo::A { b: a.b })
    }
}
