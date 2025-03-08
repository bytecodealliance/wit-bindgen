include!(env!("BINDINGS"));

pub struct MyResource;

impl exports::my::inline::foo::GuestBar for MyResource {
    fn new() -> Self {
        MyResource
    }
}

struct Component;

impl exports::my::inline::foo::Guest for Component {
    type Bar = MyResource;
}

export!(Component);
