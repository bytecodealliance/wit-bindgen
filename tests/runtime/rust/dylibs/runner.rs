//@ [lang]
//@ extern_dylibs = ["dylib_dep.rs"]

include!(env!("BINDINGS"));

use crate::my::inline::a;

struct Component;

export!(Component);

#[link(name = "dylib_dep")]
unsafe extern "C" {
    fn dylib_dep();
}

impl Guest for Component {
    fn run() {
        a::a();
        unsafe {
            dylib_dep();
        }
    }
}
