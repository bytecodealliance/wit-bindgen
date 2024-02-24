wit_bindgen::generate!({
    path: "../../tests/runtime/resource_alias",
    exports: {
        world: Test,
        "test:resource-alias/e1": E1,
        "test:resource-alias/e1/x": E1X,
        "test:resource-alias/e2": E2,
    },
});

pub struct Test {}

pub struct E1 {}

pub struct E1X(u32);

pub struct E2 {}

impl exports::test::resource_alias::e1::Guest for E1 {
    fn a(
        f: exports::test::resource_alias::e1::Foo,
    ) -> wit_bindgen::rt::vec::Vec<exports::test::resource_alias::e1::OwnX> {
        vec![f.x]
    }
}
impl exports::test::resource_alias::e1::GuestX for E1X {
    fn new(v: u32) -> Self {
        Self(v)
    }
}
impl exports::test::resource_alias::e2::Guest for E2 {
    fn a(
        f: exports::test::resource_alias::e2::Foo,
        g: exports::test::resource_alias::e2::Bar,
        _h: &exports::test::resource_alias::e1::X,
    ) -> wit_bindgen::rt::vec::Vec<exports::test::resource_alias::e2::OwnY> {
        vec![f.x, g.x]
    }
}
