include!(env!("BINDINGS"));

use crate::test::resource_with_lists::test::Thing;

fn main() {
    let thing_instance = Thing::new(b"Hi");

    assert_eq!(
        thing_instance.foo(),
        b"Hi Thing HostThing HostThing.foo Thing.foo"
    );

    thing_instance.bar(b"Hola");

    assert_eq!(
        thing_instance.foo(),
        b"Hola Thing.bar HostThing.bar HostThing.foo Thing.foo"
    );

    assert_eq!(
        Thing::baz(b"Ohayo Gozaimas"),
        b"Ohayo Gozaimas Thing.baz HostThing.baz Thing.baz again"
    );
}
