pub mod a {
    wit_bindgen::generate!({
        world: "a",
        path: "../../tests/runtime/rust_xcrate",
        pub_export_macro: true,
    });
}

pub mod b {
    wit_bindgen::generate!({
        world: "b",
        path: "../../tests/runtime/rust_xcrate",
        pub_export_macro: true,
    });
}
