//@ args = '--generate-all'

include!(env!("BINDINGS"));

struct Test;

export!(Test);

use crate::exports::i::{C, D};
use crate::exports::my::inline::bar::E;
use crate::exports::my::inline::foo::{A, B, F, G};

impl exports::my::inline::foo::Guest for Test {
    type B = B;

    fn func1(v: A) -> A {
        v
    }
    fn func2(v: B) -> B {
        v
    }
    fn func3(_: Vec<A>) -> Vec<B> {
        Vec::new()
    }
    fn func4(v: Option<A>) -> Option<A> {
        v
    }
    fn func5() -> Result<A, ()> {
        Err(())
    }
    fn func6() -> Result<F, ()> {
        Err(())
    }
    fn func7() -> Result<G, ()> {
        Err(())
    }
}

impl exports::my::inline::foo::GuestB for B {}

impl exports::my::inline::bar::Guest for Test {
    fn func6(v: E) -> E {
        v
    }
}

impl exports::i::Guest for Test {
    fn func7(a: C) -> C {
        a
    }

    fn func8(a: D) -> D {
        a
    }
}
