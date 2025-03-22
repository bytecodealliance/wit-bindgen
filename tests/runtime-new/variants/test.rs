include!(env!("BINDINGS"));

use crate::exports::test::variants::to_test::*;

struct Component;

export!(Component);

impl exports::test::variants::to_test::Guest for Component {
    
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

    fn variant_enums(a: bool, b: Result<(), ()>, c: MyErrno) -> (bool, Result<(), ()>, MyErrno) {
        (a, b, c)
    }
}
