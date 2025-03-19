//@ [lang]
//@ externs = ['rust_xcrate_test.rs']

struct Exports;

rust_xcrate_test::b::export_b!(Exports);

impl rust_xcrate_test::b::exports::test::xcrate::b_exports::Guest for Exports {
    type X = MyX;

    fn b() {
        rust_xcrate_test::a::test::xcrate::a_imports::f();
        rust_xcrate_test::b::test::xcrate::b_imports::b();

        let x = rust_xcrate_test::a::test::xcrate::a_imports::X::new();
        x.foo();
        drop(x);

        let x = rust_xcrate_test::b::test::xcrate::b_imports::X::new();
        x.foo();
        drop(x);
    }
}

struct MyX;

impl rust_xcrate_test::b::exports::test::xcrate::b_exports::GuestX for MyX {
    fn new() -> MyX {
        MyX
    }

    fn foo(&self) {}
}
