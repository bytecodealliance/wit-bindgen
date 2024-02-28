pub mod a {
    wit_bindgen::generate!({
        world: "a",
        path: "../../../tests/runtime/rust_xcrate",
        default_bindings_module: "rust_xcrate_test::a",
        pub_export_macro: true,
        export_macro_name: "export_a",
    });
}

pub mod b {
    wit_bindgen::generate!({
        world: "b",
        path: "../../../tests/runtime/rust_xcrate",
        default_bindings_module: "rust_xcrate_test::b",
        pub_export_macro: true,
        export_macro_name: "export_b",
    });
}
