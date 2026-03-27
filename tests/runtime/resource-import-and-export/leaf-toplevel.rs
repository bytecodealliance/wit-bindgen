include!(env!("BINDINGS"));

export!(Component);

struct Component;

impl Guest for Component {
    fn toplevel_export(a: Thing) -> Thing {
        a
    }
}
