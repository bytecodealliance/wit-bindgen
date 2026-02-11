//@ args = '--merge-structurally-equal-types'

include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::test::equal_types::blag::{
    Guest, GuestInputStream, Kind1, Kind3, Kind4, TStream, Tree,
};
use crate::test::equal_types::blag;

impl GuestInputStream for u32 {
    fn read(&self, _len: u64) -> Vec<u8> {
        Vec::new()
    }
}

impl Guest for Component {
    type InputStream = u32;
    fn f(x: Kind1) -> Kind1 {
        blag::f(x)
    }
    fn g(_x: Kind3) -> Kind4 {
        todo!()
    }
    fn h(x: TStream) -> Tree {
        let x = blag::TStream {
            tree: x.tree,
            stream: None,
        };
        blag::h(&x)
    }
}

use crate::exports::test::equal_types::blah::{
    CustomResult, Guest as HGuest, Kind5, Kind6, Kind7, R1, R2,
};
use crate::test::equal_types::blah;

impl HGuest for Component {
    fn f(x: Kind6) -> Kind5 {
        blah::f(x)
    }
    fn g(_x: Kind7) -> Kind4 {
        todo!()
    }
    fn h(x: TStream) -> CustomResult {
        let x = blah::TStream {
            tree: x.tree,
            stream: None,
        };
        blah::h(&x)
    }

    // Intentionally swap relative to the `*.wit` since these should generate
    // the same type.
    fn alias_type(x: R2) -> R1 {
        x
    }
}
