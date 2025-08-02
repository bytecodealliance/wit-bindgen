include!(env!("BINDINGS"));

pub struct Test {}

export!(Test);

#[allow(dead_code)]
pub struct E1X(u32);

impl exports::test::resource_alias::e1::Guest for Test {
    type X = E1X;

    fn a(f: exports::test::resource_alias::e1::Foo) -> Vec<exports::test::resource_alias::e1::X> {
        vec![f.x]
    }
}
impl exports::test::resource_alias::e1::GuestX for E1X {
    fn new(v: u32) -> Self {
        Self(v)
    }
}
impl exports::test::resource_alias::e2::Guest for Test {
    fn a(
        f: exports::test::resource_alias::e2::Foo,
        g: exports::test::resource_alias::e2::Bar,
        _h: exports::test::resource_alias::e1::XBorrow<'_>,
    ) -> Vec<exports::test::resource_alias::e2::Y> {
        vec![f.x, g.x]
    }
}
