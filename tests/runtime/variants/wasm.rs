wit_bindgen_guest_rust::import!("../../tests/runtime/variants/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/variants/exports.wit");

use exports::*;

struct Exports;

impl exports::Exports for Exports {
    fn test_imports() {
        use imports::*;

        assert_eq!(roundtrip_option(Some(1.0)), Some(1));
        assert_eq!(roundtrip_option(None), None);
        assert_eq!(roundtrip_option(Some(2.0)), Some(2));
        assert_eq!(roundtrip_result(Ok(2)), Ok(2.0));
        assert_eq!(roundtrip_result(Ok(4)), Ok(4.0));
        assert_eq!(roundtrip_result(Err(5.3)), Err(5));

        assert_eq!(roundtrip_enum(E1::A), E1::A);
        assert_eq!(roundtrip_enum(E1::B), E1::B);

        assert_eq!(invert_bool(true), false);
        assert_eq!(invert_bool(false), true);

        let (a1, a2, a3, a4, a5, a6) =
            variant_casts((C1::A(1), C2::A(2), C3::A(3), C4::A(4), C5::A(5), C6::A(6.0)));
        assert!(matches!(a1, C1::A(1)));
        assert!(matches!(a2, C2::A(2)));
        assert!(matches!(a3, C3::A(3)));
        assert!(matches!(a4, C4::A(4)));
        assert!(matches!(a5, C5::A(5)));
        assert!(matches!(a6, C6::A(b) if b == 6.0));

        let (a1, a2, a3, a4, a5, a6) = variant_casts((
            C1::B(1),
            C2::B(2.0),
            C3::B(3.0),
            C4::B(4.0),
            C5::B(5.0),
            C6::B(6.0),
        ));
        assert!(matches!(a1, C1::B(1)));
        assert!(matches!(a2, C2::B(b) if b == 2.0));
        assert!(matches!(a3, C3::B(b) if b == 3.0));
        assert!(matches!(a4, C4::B(b) if b == 4.0));
        assert!(matches!(a5, C5::B(b) if b == 5.0));
        assert!(matches!(a6, C6::B(b) if b == 6.0));

        let (a1, a2, a3, a4) = variant_zeros((Z1::A(1), Z2::A(2), Z3::A(3.0), Z4::A(4.0)));
        assert!(matches!(a1, Z1::A(1)));
        assert!(matches!(a2, Z2::A(2)));
        assert!(matches!(a3, Z3::A(b) if b == 3.0));
        assert!(matches!(a4, Z4::A(b) if b == 4.0));

        let (a1, a2, a3, a4) = variant_zeros((Z1::B, Z2::B, Z3::B, Z4::B));
        assert!(matches!(a1, Z1::B));
        assert!(matches!(a2, Z2::B));
        assert!(matches!(a3, Z3::B));
        assert!(matches!(a4, Z4::B));

        variant_typedefs(None, false, Err(()));

        assert_eq!(
            variant_enums(true, Ok(()), MyErrno::Success),
            (false, Err(()), MyErrno::A)
        );
    }

    fn roundtrip_option(a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(a: bool) -> bool {
        !a
    }

    fn variant_casts(a: Casts) -> Casts {
        a
    }

    fn variant_zeros(a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(_: Option<u32>, _: bool, _: Result<u32, ()>) {}
}
