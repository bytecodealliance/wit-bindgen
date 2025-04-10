include!(env!("BINDINGS"));

use crate::my::inline::foo::Bar;

fn main() {
    let _ = Bar::new();
}
