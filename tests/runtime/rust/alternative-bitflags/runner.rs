//@ args = '--bitflags-path crate::my_bitflags'

include!(env!("BINDINGS"));

pub(crate) use wit_bindgen::rt::bitflags as my_bitflags;

use crate::my::inline::t::{get_flag, Bar};

fn main() {
    assert_eq!(get_flag(), Bar::BAZ);
}
