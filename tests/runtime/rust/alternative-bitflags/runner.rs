//@ args = '--bitflags-path crate::my_bitflags'

include!(env!("BINDINGS"));

pub(crate) use wit_bindgen::rt::bitflags as my_bitflags;

use crate::my::inline::t::{get_flag, Bar};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        assert_eq!(get_flag(), Bar::BAZ);
    }
}
