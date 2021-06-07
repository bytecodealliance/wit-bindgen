#![allow(dead_code)]

fn main() {
    println!("compiled successfully!")
}

test_codegen::test_rust_codegen!();

fn smoke_export() -> &'static impl smoke_export::SmokeExport {
    struct A;
    impl smoke_export::SmokeExport for A {
        fn y(&self) {}
    }
    &A
}
