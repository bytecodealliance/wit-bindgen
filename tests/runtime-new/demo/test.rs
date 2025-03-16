include!(env!("BINDINGS"));

export!(Test);

struct Test;

impl exports::a::b::the_test::Guest for Test {
    fn x() {}
}
