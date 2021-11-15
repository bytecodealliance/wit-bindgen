wai_bindgen_rust::import!("crates/nested_b/nested_b.wai");

use nested_b::*;

fn main() {
    assert_eq!(outer("you spin me right round"), "you spin me right round");
}
