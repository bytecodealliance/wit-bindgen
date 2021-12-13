wit_bindgen_rust::import!("crates/flags/flags.wit");

use flags::*;

fn main() {
    assert_eq!(roundtrip_flag1(Flag1::empty()), Flag1::empty());
    assert_eq!(roundtrip_flag1(Flag1::B0), Flag1::B0);

    assert_eq!(roundtrip_flag2(Flag2::empty()), Flag2::empty());
    assert_eq!(roundtrip_flag2(Flag2::B0), Flag2::B0);
    assert_eq!(roundtrip_flag2(Flag2::B1 | Flag2::B0), Flag2::B1 | Flag2::B0);

    assert_eq!(roundtrip_flag4(Flag4::empty()), Flag4::empty());
    assert_eq!(roundtrip_flag4(Flag4::B0), Flag4::B0);
    assert_eq!(roundtrip_flag4(Flag4::B1 | Flag4::B0), Flag4::B1 | Flag4::B0);
    assert_eq!(
        roundtrip_flag4(Flag4::B2 | Flag4::B1 | Flag4::B0),
        Flag4::B2 | Flag4::B1 | Flag4::B0
    );
    assert_eq!(
        roundtrip_flag4(Flag4::B3 | Flag4::B2 | Flag4::B1 | Flag4::B0),
        Flag4::B3 | Flag4::B2 | Flag4::B1 | Flag4::B0
    );
}
