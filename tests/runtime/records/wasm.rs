wit_bindgen_guest_rust::generate!("world" in "../../tests/runtime/records");

use exports::*;

struct Component;

export_records!(Component);

impl Records for Component {
    fn test_imports() {
        use imports::*;

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
            roundtrip_flags3(Flag8::B0, Flag16::B1, Flag32::B2, Flag64::B3),
            (Flag8::B0, Flag16::B1, Flag32::B2, Flag64::B3)
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

        assert_eq!(tuple0(()), ());
        assert_eq!(tuple1((1,)), (1,));
    }
}

impl Exports for Component {
    fn multiple_results() -> (u8, u16) {
        (100, 200)
    }

    fn swap_tuple(a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(a: F1) -> F1 {
        a
    }

    fn roundtrip_flags2(a: F2) -> F2 {
        a
    }

    fn roundtrip_flags3(
        a: Flag8,
        b: Flag16,
        c: Flag32,
        d: Flag64,
    ) -> (Flag8, Flag16, Flag32, Flag64) {
        (a, b, c, d)
    }

    fn roundtrip_record1(a: R1) -> R1 {
        a
    }

    fn tuple0(_: ()) {}

    fn tuple1(a: (u8,)) -> (u8,) {
        (a.0,)
    }
}
