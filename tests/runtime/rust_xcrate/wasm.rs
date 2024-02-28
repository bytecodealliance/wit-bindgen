// This is a test where the cross-crate-behavior of the `generate!` macro is
// tested.
//
// Specifically the `rust_xcrate_test` test, located at
// `crates/test-rust-wasm/rust-xcrate-test`, generates bindings for this WIT
// package in this directory. The WIT package contains three worlds: `a`, `b`,
// and `c`. This crate will generate bindings for `b`, but also use imports from
// `a`, effectively using world `c` when everything is union'd together. The
// host then expects world `c`.
//
// This ensures that the exports of `a`, which are never defined, are not
// accidentally looked for during the componentization process.

struct Exports;

rust_xcrate_test::b::export_b!(Exports);

impl rust_xcrate_test::b::Guest for Exports {
    fn b() {
        rust_xcrate_test::a::test::xcrate::a_imports::a();
        rust_xcrate_test::b::test::xcrate::b_imports::b();

        let x = rust_xcrate_test::a::test::xcrate::a_imports::X::new();
        x.foo();
        drop(x);

        let x = rust_xcrate_test::b::test::xcrate::b_imports::X::new();
        x.foo();
        drop(x);
    }
}

impl rust_xcrate_test::b::exports::an_exported_interface::Guest for Exports {
    type X = MyX;
}

struct MyX;

impl rust_xcrate_test::b::exports::an_exported_interface::GuestX for MyX {
    fn new() -> MyX {
        MyX
    }

    fn foo(&self) {}
}
