wit_bindgen_rust::import!("../resources/resources.wit");

use resources::*;

fn main() {
    // This should trap in the runtime as there are no valid resource handles.
    receive_an_x(unsafe { &X::from_raw(0) });
}
