include!(env!("BINDINGS"));

struct Component;
export!(Component);

impl exports::a::b::x::Guest for Component {
    fn f1() {}
}

impl exports::other::c::x::Guest for Component {
    fn f2() {}
}
