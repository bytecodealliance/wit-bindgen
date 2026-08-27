wit_bindgen::generate!("runner-dep" in "test.wit");

use crate::my::inline::b;

#[unsafe(no_mangle)]
pub extern "C" fn dylib_dep() {
    b::b();
}
