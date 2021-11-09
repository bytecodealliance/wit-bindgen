wai_bindgen_rust::export!("crates/flags/flags.wai");

use flags::*;

struct Flags;

impl flags::Flags for Flags {
    fn roundtrip_flag1(x: Flag1) -> Flag1 {
        x
    }
    fn roundtrip_flag2(x: Flag2) -> Flag2 {
        x
    }
    fn roundtrip_flag4(x: Flag4) -> Flag4 {
        x
    }
    fn roundtrip_flag8(x: Flag8) -> Flag8 {
        x
    }
    fn roundtrip_flag16(x: Flag16) -> Flag16 {
        x
    }
    fn roundtrip_flag32(x: Flag32) -> Flag32 {
        x
    }
    fn roundtrip_flag64(x: Flag64) -> Flag64 {
        x
    }
}
