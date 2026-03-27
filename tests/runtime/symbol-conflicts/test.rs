include!(env!("BINDINGS"));

struct Component;

impl exports::my::inline::foo1::Guest for Component {
    fn foo() {}
}

impl exports::my::inline::foo2::Guest for Component {
    fn foo() {}
}

impl exports::my::inline::bar1::Guest for Component {
    fn bar() -> String {
        String::new()
    }
}

impl exports::my::inline::bar2::Guest for Component {
    fn bar() -> String {
        String::new()
    }
}

export!(Component);
