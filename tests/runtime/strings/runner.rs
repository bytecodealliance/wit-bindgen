include!(env!("BINDINGS"));

use crate::test::strings::to_test::*;

fn main() {
    take_basic("latin utf16");
    assert_eq!(return_unicode(), "🚀🚀🚀 𠈄𓀀");
    assert_eq!(return_empty(), "");
    assert_eq!(roundtrip("🚀🚀🚀 𠈄𓀀"), "🚀🚀🚀 𠈄𓀀");
}
