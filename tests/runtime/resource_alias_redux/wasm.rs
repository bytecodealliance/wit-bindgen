wit_bindgen::generate!({
    path: "../../tests/runtime/resource_alias_redux",
});

use test::resource_alias_redux::resource_alias1::{
    a as import_a, Foo as ImportAlias1Foo, Thing as ImportThing,
};
use test::resource_alias_redux::resource_alias2::{b as import_b, Foo as ImportAlias2Foo};

pub struct Test {}

export!(Test);

pub struct MyThing {
    value: Option<ImportThing>,
}

impl exports::test::resource_alias_redux::resource_alias1::Guest for Test {
    type Thing = MyThing;

    fn a(
        mut f: exports::test::resource_alias_redux::resource_alias1::Foo,
    ) -> Vec<exports::test::resource_alias_redux::resource_alias1::Thing> {
        let foo = ImportAlias1Foo {
            thing: Option::take(&mut f.thing.get_mut::<MyThing>().value).unwrap(),
        };
        import_a(foo)
            .into_iter()
            .map(|t| {
                exports::test::resource_alias_redux::resource_alias1::Thing::new(MyThing {
                    value: Some(t),
                })
            })
            .collect()
    }
}
impl exports::test::resource_alias_redux::resource_alias1::GuestThing for MyThing {
    fn new(s: String) -> Self {
        Self {
            value: Some(ImportThing::new(&(s + " Thing"))),
        }
    }
    fn get(&self) -> String {
        self.value.as_ref().unwrap().get() + " Thing.get"
    }
}

impl exports::test::resource_alias_redux::resource_alias2::Guest for Test {
    fn b(
        mut f: exports::test::resource_alias_redux::resource_alias2::Foo,
        mut g: exports::test::resource_alias_redux::resource_alias2::Bar,
    ) -> Vec<exports::test::resource_alias_redux::resource_alias2::Thing> {
        let foo = ImportAlias2Foo {
            thing: Option::take(&mut f.thing.get_mut::<MyThing>().value).unwrap(),
        };
        let bar = ImportAlias1Foo {
            thing: Option::take(&mut g.thing.get_mut::<MyThing>().value).unwrap(),
        };
        import_b(foo, bar)
            .into_iter()
            .map(|t| {
                exports::test::resource_alias_redux::resource_alias1::Thing::new(MyThing {
                    value: Some(t),
                })
            })
            .collect()
    }
}
impl Guest for Test {
    fn test(things: Vec<Thing>) -> Vec<Thing> {
        things
    }
}
