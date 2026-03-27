include!(env!("BINDINGS"));

pub struct MyResource {
    data: u32,
}

impl exports::my::inline::foo::GuestBar for MyResource {
    fn new(data: u32) -> Self {
        Self { data }
    }

    fn get_data(&self) -> u32 {
        self.data
    }

    fn consume(mut this: exports::my::inline::foo::Bar) -> u32 {
        let me: &MyResource = this.get();
        let prior_data: &u32 = &me.data;
        let new_data = prior_data + 1;
        let me: &mut MyResource = this.get_mut();
        let mutable_data: &mut u32 = &mut me.data;
        *mutable_data = new_data;
        me.data
    }
}

struct Component;

impl exports::my::inline::foo::Guest for Component {
    type Bar = MyResource;
}

export!(Component);
