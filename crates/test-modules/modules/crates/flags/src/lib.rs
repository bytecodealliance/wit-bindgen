witx_bindgen_rust::export!("crates/flags/flags.witx");

struct Component;

use flags::*;

impl Flags for Component {
    fn roundtrip_flag1(&self, x: Flag1) -> Flag1 {
        x
    }
    fn roundtrip_flag2(&self, x: Flag2) -> Flag2 {
        x
    }
    fn roundtrip_flag4(&self, x: Flag4) -> Flag4 {
        x
    }
    fn roundtrip_flag8(&self, x: Flag8) -> Flag8 {
        x
    }
    fn roundtrip_flag16(&self, x: Flag16) -> Flag16 {
        x
    }
    fn roundtrip_flag32(&self, x: Flag32) -> Flag32 {
        x
    }
    fn roundtrip_flag64(&self, x: Flag64) -> Flag64 {
        x
    }
}

fn flags() -> &'static impl Flags {
    static INSTANCE: Component = Component;
    &INSTANCE
}
