include!(env!("BINDINGS"));

use crate::test::records::to_test::*;

fn main() {
    assert_eq!(multiple_results(), (4, 5));

    assert_eq!(swap_tuple((1u8, 2u32)), (2u32, 1u8));
    assert_eq!(roundtrip_flags1(F1::A), F1::A);
    assert_eq!(roundtrip_flags1(F1::empty()), F1::empty());
    assert_eq!(roundtrip_flags1(F1::B), F1::B);
    assert_eq!(roundtrip_flags1(F1::A | F1::B), F1::A | F1::B);

    assert_eq!(roundtrip_flags2(F2::C), F2::C);
    assert_eq!(roundtrip_flags2(F2::empty()), F2::empty());
    assert_eq!(roundtrip_flags2(F2::D), F2::D);
    assert_eq!(roundtrip_flags2(F2::C | F2::E), F2::C | F2::E);

    assert_eq!(
        roundtrip_flags3(Flag8::B0, Flag16::B1, Flag32::B2),
        (Flag8::B0, Flag16::B1, Flag32::B2)
    );

    let r = roundtrip_record1(R1 {
        a: 8,
        b: F1::empty(),
    });
    assert_eq!(r.a, 8);
    assert_eq!(r.b, F1::empty());

    let r = roundtrip_record1(R1 {
        a: 0,
        b: F1::A | F1::B,
    });
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1::A | F1::B);

    assert_eq!(tuple1((1,)), (1,));
}