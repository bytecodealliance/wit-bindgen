wit_bindgen::generate!({
    path: "../../tests/runtime/type_section_suffix",
    world: "required-exports",
});

export!(Exports);

// generate bindings once here
mod a {
    wit_bindgen::generate!(
        "available-imports" in "../../tests/runtime/type_section_suffix"
    );
}

// generate bindings again for the same world, this time using a different
// suffix
mod b {
    wit_bindgen::generate!({
        world: "available-imports",
        path: "../../tests/runtime/type_section_suffix",
        type_section_suffix: "hello i am a suffix how are you doing today",
    });
}

mod c {
    wit_bindgen::generate!({
        world: "test:a/imports",
        path: "../../tests/runtime/type_section_suffix",
    });
}
mod d {
    wit_bindgen::generate!({
        world: "test:b/imports",
        path: "../../tests/runtime/type_section_suffix",
    });
}

struct Exports;

impl Guest for Exports {
    fn run() {
        a::test::suffix::imports::foo();
        b::test::suffix::imports::foo();
        c::foo();
        d::bar();
    }
}
