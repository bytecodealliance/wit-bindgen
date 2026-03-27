//@ args = '--merge-structurally-equal-types'

include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::test::equal_types::blag::{
    Guest, GuestInputStream, InputStream, Kind1, Kind3, Kind4, TStream, Tree,
};

impl GuestInputStream for u32 {
    fn read(&self, _len: u64) -> Vec<u8> {
        Vec::new()
    }
}

impl Guest for Component {
    type InputStream = u32;
    fn f(x: Kind1) -> Kind1 {
        x
    }
    fn g(x: Kind3) -> Kind4 {
        Kind4 { a: x.a }
    }
    fn h(x: TStream) -> Tree {
        x.tree
    }
}

use crate::exports::test::equal_types::blah::{
    CustomResult, Guest as HGuest, Kind5, Kind6, Kind7, R1, R2,
};

impl HGuest for Component {
    fn f(x: Kind6) -> Kind5 {
        match x {
            Kind6::A => Kind1::A,
            Kind6::B(x) => Kind5::B(x),
            Kind6::C => Kind1::C,
        }
    }
    fn g(x: Kind7) -> Kind4 {
        Kind4 {
            a: InputStream::new(*x.a.get::<u32>()),
        }
    }
    fn h(x: TStream) -> CustomResult {
        CustomResult::Ok(x.tree)
    }

    // Intentionally swap relative to the `*.wit` since these should generate
    // the same type.
    fn alias_type(x: R2) -> R1 {
        x
    }
}

const _: () = {
    use crate::exports::test::equal_types::resources::*;

    struct GuestResource;

    impl Guest for Component {
        type R1 = GuestResource;

        // Intentionally swap the 1/2 relative to WIT to ensure that the types
        // are equivalent.
        fn alias_own(x: T2) -> T1 {
            x
        }
        fn alias_aggregate(x: Option<T2>) -> Option<T1> {
            x
        }
    }

    impl GuestR1 for GuestResource {}
};
