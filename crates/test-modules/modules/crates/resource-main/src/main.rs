witx_bindgen_rust::import!("../../../tests/resource.witx");

use resource::*;

fn main() {
    receive_an_x(&acquire_an_x());
}
