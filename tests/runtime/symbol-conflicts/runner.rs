include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        my::inline::foo1::foo();
        my::inline::foo2::foo();
        my::inline::bar1::bar();
        my::inline::bar2::bar();
    }
}
