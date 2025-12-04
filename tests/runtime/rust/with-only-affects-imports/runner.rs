include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        my::inline::foo::bar(my::inline::foo::A { b: 2 });
    }
}
