use crate::test::resource_import_and_export::test::Thing;

include!(env!("BINDINGS"));

fn main() {
    let thing1 = Thing::new(42);

    // 42 + 1 (constructor) + 1 (constructor) + 2 (foo) + 2 (foo)
    assert_eq!(thing1.foo(), 48);

    // 33 + 3 (bar) + 3 (bar) + 2 (foo) + 2 (foo)
    thing1.bar(33);
    assert_eq!(thing1.foo(), 43);

    let thing2 = Thing::new(81);
    let thing3 = Thing::baz(thing1, thing2);
    assert_eq!(
        thing3.foo(),
        33 + 3 + 3 + 81 + 1 + 1 + 2 + 2 + 4 + 1 + 2 + 4 + 1 + 1 + 2 + 2
    );
}
