wai_bindgen_rust::import!("crates/flags/flags.wai");

use flags::*;

fn main() {
    assert_eq!(roundtrip_flag1(0), 0);
    assert_eq!(roundtrip_flag1(FLAG1_B0), FLAG1_B0);

    assert_eq!(roundtrip_flag2(0), 0);
    assert_eq!(roundtrip_flag2(FLAG2_B0), FLAG2_B0);
    assert_eq!(roundtrip_flag2(FLAG2_B1 | FLAG2_B0), FLAG2_B1 | FLAG2_B0);

    assert_eq!(roundtrip_flag4(0), 0);
    assert_eq!(roundtrip_flag4(FLAG4_B0), FLAG4_B0);
    assert_eq!(roundtrip_flag4(FLAG4_B1 | FLAG4_B0), FLAG4_B1 | FLAG4_B0);
    assert_eq!(
        roundtrip_flag4(FLAG4_B2 | FLAG4_B1 | FLAG4_B0),
        FLAG4_B2 | FLAG4_B1 | FLAG4_B0
    );
    assert_eq!(
        roundtrip_flag4(FLAG4_B3 | FLAG4_B2 | FLAG4_B1 | FLAG4_B0),
        FLAG4_B3 | FLAG4_B2 | FLAG4_B1 | FLAG4_B0
    );
}
