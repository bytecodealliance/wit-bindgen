include!(env!("BINDINGS"));

use crate::exports::test::records::to_test::*;

pub struct Test {}

export!(Test);

impl Guest for Test {
    fn multiple_results() -> (u8, u16) {
        (4, 5)
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

    fn roundtrip_flags3(a: Flag8, b: Flag16, c: Flag32) -> (Flag8, Flag16, Flag32) {
        (a, b, c)
    }

    fn roundtrip_record1(a: R1) -> R1 {
        a
    }

    fn tuple1(a: (u8,)) -> (u8,) {
        (a.0,)
    }
}