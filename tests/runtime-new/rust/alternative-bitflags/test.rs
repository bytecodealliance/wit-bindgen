//@ args = '--bitflags-path crate::my_bitflags'

include!(env!("BINDINGS"));

pub(crate) use wit_bindgen::bitflags as my_bitflags;

struct Component;

export!(Component);

use crate::exports::my::inline::t::{Bar, Guest};

impl Guest for Component {
    fn get_flag() -> Bar {
        Bar::BAZ
    }
}
