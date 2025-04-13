//@ args = '--with my:inline/foo=other::my::inline::foo'

include!(env!("BINDINGS"));

mod other {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                resource a;

                bar: func() -> a;
            }

            world dummy {
                import foo;
            }
        ",
    });
}

fn main() {
    let resource = other::my::inline::foo::bar();
    my::inline::bar::bar(resource);
}
