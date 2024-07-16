// This is a test where the cross-crate-behavior of the `generate!` macro is
// tested.
//
// Specifically the `test_rust_wasm::rust_bincrate` test, located at
// `crates/test-rust-wasm/rust-xcrate-test`, generates bindings for this WIT
// package in this directory. The WIT package contains three worlds: `a`, `b`,
// and `c`. This crate will generate bindings for `b`, but also use imports from
// `a`, effectively using world `c` when everything is union'd together. The
// host then expects world `c`.
//
// This ensures that the exports of `a`, which are never defined, are not
// accidentally looked for during the componentization process.

struct Exports;

test_rust_wasm::rust_bincrate::b::export!(Exports);

impl test_rust_wasm::rust_bincrate::b::Guest for Exports {
    fn b() {
        test_rust_wasm::rust_bincrate::a::test::xcrate::a_imports::a();
        test_rust_wasm::rust_bincrate::b::test::xcrate::b_imports::b();

        let x = test_rust_wasm::rust_bincrate::a::test::xcrate::a_imports::X::new();
        x.foo();
        drop(x);

        let x = test_rust_wasm::rust_bincrate::b::test::xcrate::b_imports::X::new();
        x.foo();
        drop(x);
    }
}

impl test_rust_wasm::rust_bincrate::b::exports::an_exported_interface::Guest for Exports {
    type X = MyX;
}

struct MyX;

impl test_rust_wasm::rust_bincrate::b::exports::an_exported_interface::GuestX for MyX {
    fn new() -> MyX {
        MyX
    }

    fn foo(&self) {}
}

fn main() {}
