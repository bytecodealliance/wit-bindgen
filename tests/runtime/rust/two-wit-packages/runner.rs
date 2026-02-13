include!(env!("BINDINGS"));

struct Component;
export!(Component);

impl Guest for Component {
    fn run() {
        a::b::x::f1();
        other::c::x::f2();
    }
}
