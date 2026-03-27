include!(env!("BINDINGS"));

use crate::my::inline::foo::Bar;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let _ = Bar::new();
    }
}
