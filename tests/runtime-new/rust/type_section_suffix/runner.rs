include!(env!("BINDINGS"));

// generate bindings once here
mod a {
    wit_bindgen::generate!({
        world: "available-imports",
        path: "./test.wit",
        generate_all,
    });
}

// generate bindings again for the same world, this time using a different
// suffix
mod b {
    wit_bindgen::generate!({
        world: "available-imports",
        path: "./test.wit",
        type_section_suffix: "hello i am a suffix how are you doing today",
        generate_all,
    });
}

mod c {
    wit_bindgen::generate!({
        world: "test:a/imports",
        path: "./test.wit",
    });
}
mod d {
    wit_bindgen::generate!({
        world: "test:b/imports",
        path: "./test.wit",
    });
}

fn main() {
    a::test::suffix::imports::foo();
    b::test::suffix::imports::foo();
    c::foo::f();
    d::bar::f();
}
