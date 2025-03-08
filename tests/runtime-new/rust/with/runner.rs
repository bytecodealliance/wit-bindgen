//@ args = '--with my:inline/foo=other::my::inline::foo'

include!(env!("BINDINGS"));

mod other {
    wit_bindgen::generate!({
        inline: "
            package my:inline;

            interface foo {
                record msg {
                    field: string,
                }
            }

            world dummy {
                use foo.{msg};
                import bar: func(m: msg);
            }
        ",
    });
}

fn main() {
    let msg = other::my::inline::foo::Msg {
        field: "hello".to_string(),
    };
    my::inline::bar::bar(&msg);
}
