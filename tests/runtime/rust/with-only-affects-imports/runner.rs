include!(env!("BINDINGS"));

fn main() {
    my::inline::foo::bar(my::inline::foo::A { b: 2 });
}
