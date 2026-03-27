include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::my::inline::bar::Guest as GuestBar;
use crate::exports::my::inline::foo::{Guest as GuestFoo, GuestA, A};

struct MyA;

impl GuestFoo for Component {
    type A = MyA;

    fn bar() -> A {
        A::new(MyA)
    }
}

impl GuestA for MyA {}

impl GuestBar for Component {
    fn bar(m: A) -> Vec<A> {
        vec![m]
    }
}
