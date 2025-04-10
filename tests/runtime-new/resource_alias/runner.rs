include!(env!("BINDINGS"));

use test::resource_alias::e1::{a as a1, Foo as Foo1, X};
use test::resource_alias::e2::{a as a2, Foo as Foo2};

fn main() {
    let foo_e1 = Foo1 { x: X::new(42) };
    a1(foo_e1);

    let foo_e2 = Foo2 { x: X::new(7) };
    let bar_e2 = Foo1 { x: X::new(8) };
    let y = X::new(8);
    a2(foo_e2, bar_e2, &y);
}
