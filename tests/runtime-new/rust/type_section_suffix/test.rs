include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl exports::bar::Guest for Component {
    fn f() {}
}

impl exports::foo::Guest for Component {
    fn f() {}
}

impl exports::test::suffix::imports::Guest for Component {
    fn foo() {}
}
