wit_bindgen::generate!({
    path: "../../tests/runtime/resource_alias_redux",
    exports: {
        world: Test,
        "test:resource-alias-redux/test": Test,
        "test:resource-alias-redux/resource-alias1": MyResourceAlias1,
        "test:resource-alias-redux/resource-alias2": MyResourceAlias2,
        "test:resource-alias-redux/resource-alias1/thing": MyThing,
    }
});

use test::resource_alias_redux::resource_alias1::{Thing as ImportThing, a as import_a, Foo as ImportAlias1Foo};
use test::resource_alias_redux::resource_alias2::{b as import_b, Foo as ImportAlias2Foo};

pub struct Test {}

pub struct MyResourceAlias1 {}

pub struct MyResourceAlias2 {}

pub struct MyThing {
    value: Option<ImportThing>
}

impl exports::test::resource_alias_redux::resource_alias1::Guest for MyResourceAlias1 {
    fn a(
        mut f: exports::test::resource_alias_redux::resource_alias1::Foo,
    ) -> wit_bindgen::rt::vec::Vec<exports::test::resource_alias_redux::resource_alias1::OwnThing>
    {
        let foo = ImportAlias1Foo {
            thing: Option::take(&mut f.thing.value).unwrap()
        };
        import_a(foo).into_iter().map(|t| {
            exports::test::resource_alias_redux::resource_alias1::OwnThing::new(MyThing {
                value: Some(t)
            }) 
        }).collect()
    }
}
impl exports::test::resource_alias_redux::resource_alias1::GuestThing for MyThing {
    fn new(s: wit_bindgen::rt::string::String) -> Self {
        Self {
            value: Some(ImportThing::new(&(s + " Thing")))
        }
    }
    fn get(&self) -> wit_bindgen::rt::string::String {
        self.value.as_ref().unwrap().get() + " Thing.get"
    }
}
impl exports::test::resource_alias_redux::resource_alias2::Guest for MyResourceAlias2 {
    fn b(
        mut f: exports::test::resource_alias_redux::resource_alias2::Foo,
        mut g: exports::test::resource_alias_redux::resource_alias2::Bar,
    ) -> wit_bindgen::rt::vec::Vec<exports::test::resource_alias_redux::resource_alias2::OwnThing>
    {
        let foo = ImportAlias2Foo {
            thing: Option::take(&mut f.thing.value).unwrap()
        };
        let bar = ImportAlias1Foo {
            thing: Option::take(&mut g.thing.value).unwrap()
        };
        import_b(foo, bar).into_iter().map(|t| {
            exports::test::resource_alias_redux::resource_alias1::OwnThing::new(MyThing {
                value: Some(t)
            }) 
        }).collect()
    }
}
impl Guest for Test {
    fn test(things: wit_bindgen::rt::vec::Vec<Thing>) -> wit_bindgen::rt::vec::Vec<Thing> {
        things
    }
}
