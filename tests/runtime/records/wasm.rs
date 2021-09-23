witx_bindgen_rust::import!("./tests/runtime/records/imports.witx");
witx_bindgen_rust::export!("./tests/runtime/records/exports.witx");

use exports::*;

struct Exports;

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;

        assert_eq!(multiple_results(), (4, 5));

        assert_eq!(swap_tuple((1u8, 2u32)), (2u32, 1u8));
        assert_eq!(roundtrip_flags1(F1_A), F1_A);
        assert_eq!(roundtrip_flags1(0), 0);
        assert_eq!(roundtrip_flags1(F1_B), F1_B);
        assert_eq!(roundtrip_flags1(F1_A | F1_B), F1_A | F1_B);

        assert_eq!(roundtrip_flags2(F2_C), F2_C);
        assert_eq!(roundtrip_flags2(0), 0);
        assert_eq!(roundtrip_flags2(F2_D), F2_D);
        assert_eq!(roundtrip_flags2(F2_C | F2_E), F2_C | F2_E);

        assert_eq!(
            roundtrip_flags3(FLAG8_B0, FLAG16_B1, FLAG32_B2, FLAG64_B3),
            (FLAG8_B0, FLAG16_B1, FLAG32_B2, FLAG64_B3)
        );

        let r = roundtrip_record1(R1 { a: 8, b: 0 });
        assert_eq!(r.a, 8);
        assert_eq!(r.b, 0);

        let r = roundtrip_record1(R1 {
            a: 0,
            b: F1_A | F1_B,
        });
        assert_eq!(r.a, 0);
        assert_eq!(r.b, F1_A | F1_B);

        assert_eq!(tuple0(()), ());
        assert_eq!(tuple1((1,)), (1,));
    }

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

    fn roundtrip_flags3(a: F8, b: F16, c: F32, d: F64) -> (F8, F16, F32, F64) {
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
