include!(env!("BINDINGS"));

use crate::my::inline::foo::Bar;

fn main() {
    let data = Bar::new(3);
    assert_eq!(data.get_data(), 3);
    assert_eq!(Bar::consume(data), 4);
}
