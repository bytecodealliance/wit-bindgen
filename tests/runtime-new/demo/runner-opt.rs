//@ [lang]
//@ rustflags = '-O'

include!(env!("BINDINGS"));

fn main() {
    a::b::the_test::x();
}
