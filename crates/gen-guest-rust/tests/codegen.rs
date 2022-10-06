#![allow(dead_code, type_alias_bounds)]

#[test]
fn ok() {}

#[rustfmt::skip]
mod imports {
    test_helpers::codegen_rust_wasm_import!(
        "*.wit"

        // If you want to exclude a specific test you can include it here with
        // gitignore glob syntax:
        //
        // "!wasm.wit"
        // "!host.wit"
        //
        //
        // Similarly you can also just remove the `*.wit` glob and list tests
        // individually if you're debugging.
    );
}

#[rustfmt::skip]
mod exports {
    test_helpers::codegen_rust_wasm_export!(
        "*.wit"
    );
}

mod strings {
    wit_bindgen_guest_rust::import!({
        src["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
    });

    fn test() {
        // Test the argument is `&str`.
        cat::foo("hello");

        // Test the return type is `String`.
        let _t: String = cat::bar();
    }
}

/// Like `strings` but with raw_strings`.
mod raw_strings {
    wit_bindgen_guest_rust::import!({
        src["cat"]: "
            foo: func(x: string)
            bar: func() -> string
        ",
        raw_strings,
    });

    fn test() {
        // Test the argument is `&[u8]`.
        cat::foo(b"hello");

        // Test the return type is `Vec<u8>`.
        let _t: Vec<u8> = cat::bar();
    }
}
