witx_bindgen_rust::import!("crates/nested_b/nested_b.witx");

use nested_b::*;

fn main() {
    assert_eq!(outer("you spin me right round"), "you spin me right round");
}
