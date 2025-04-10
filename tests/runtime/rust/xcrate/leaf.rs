include!(env!("BINDINGS"));

export!(Component);

struct Component;

struct MyX;

impl crate::exports::test::xcrate::a_imports::Guest for Component {
    type X = MyX;

    fn f() {}
}

impl crate::exports::test::xcrate::a_imports::GuestX for MyX {
    fn new() -> MyX {
        MyX
    }

    fn foo(&self) {}
}

impl crate::exports::test::xcrate::b_imports::Guest for Component {
    type X = MyX;

    fn b() {}
}

impl crate::exports::test::xcrate::b_imports::GuestX for MyX {
    fn new() -> MyX {
        MyX
    }

    fn foo(&self) {}
}
