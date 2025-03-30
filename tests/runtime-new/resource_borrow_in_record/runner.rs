include!(env!("BINDINGS"));

use crate::test::resource_borrow_in_record::to_test::{test, Foo, Thing};

fn main() {
    let thing1 = Thing::new("Bonjour");
    let thing2 = Thing::new("mon cher");
    let result = test(&[Foo { thing: &thing1 }, Foo { thing: &thing2 }])
        .into_iter()
        .map(|x| x.get())
        .collect::<Vec<_>>();
    assert_eq!(result, ["Bonjour new test get", "mon cher new test get"]);
}
