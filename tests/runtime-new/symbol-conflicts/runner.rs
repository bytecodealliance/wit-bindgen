include!(env!("BINDINGS"));

fn main() {
    my::inline::foo1::foo();
    my::inline::foo2::foo();
    my::inline::bar1::bar();
    my::inline::bar2::bar();
}
