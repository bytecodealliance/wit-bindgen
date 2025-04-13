include!(env!("BINDINGS"));

use crate::test::resource_alias_redux::resource_alias1 as a1;
use crate::test::resource_alias_redux::resource_alias2 as a2;
use crate::the_test::test;

fn main() {
    let thing1 = crate::the_test::Thing::new("Ni Hao");
    let result = test(vec![thing1]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].get(), "Ni Hao GuestThing GuestThing.get");

    let thing2 = crate::test::resource_alias_redux::resource_alias1::Thing::new("Ciao");
    let result = a1::a(a1::Foo { thing: thing2 });
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].get(), "Ciao GuestThing GuestThing.get");

    let thing3 = crate::test::resource_alias_redux::resource_alias1::Thing::new("Ciao");
    let thing4 = crate::test::resource_alias_redux::resource_alias1::Thing::new("Aloha");

    let result = a2::b(a2::Foo { thing: thing3 }, a2::Bar { thing: thing4 });
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].get(), "Ciao GuestThing GuestThing.get");
    assert_eq!(result[1].get(), "Aloha GuestThing GuestThing.get");
}
