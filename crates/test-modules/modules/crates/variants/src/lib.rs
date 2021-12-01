wit_bindgen_rust::export!("crates/variants/variants.wit");

use variants::*;

struct Variants;

impl variants::Variants for Variants {
    fn e1_arg(x: E1) {
        match x {
            E1::A => {}
        }
    }
    fn e1_result() -> E1 {
        E1::A
    }
    fn u1_arg(x: U1) {
        match x {
            U1::V0(x) => assert_eq!(x, 1234),
            _ => panic!(),
        }
    }
    fn u1_result() -> U1 {
        U1::V1(432.1)
    }
    fn v1_arg(x: V1) {
        match x {
            V1::D(x) => assert_eq!(x, "hello world!"),
            _ => panic!(),
        }
    }
    fn v1_result() -> V1 {
        V1::G(54321)
    }
    fn bool_arg(x: bool) {
        assert!(x);
    }
    fn bool_result() -> bool {
        false
    }
    fn option_arg(
        a: Option<bool>,
        b: Option<()>,
        c: Option<u32>,
        d: Option<E1>,
        e: Option<f32>,
        f: Option<U1>,
        g: Option<Option<bool>>,
    ) {
        assert_eq!(a, None);
        assert_eq!(b, Some(()));
        assert_eq!(c, Some(12345));
        match d {
            Some(E1::A) => {}
            _ => panic!(),
        }
        assert_eq!(e, None);
        match f {
            Some(U1::V0(67890)) => {}
            _ => panic!(),
        }
        assert_eq!(g, Some(Some(true)));
    }
    fn option_result() -> (
        Option<bool>,
        Option<()>,
        Option<u32>,
        Option<E1>,
        Option<f32>,
        Option<U1>,
        Option<Option<bool>>,
    ) {
        (
            Some(false),
            None,
            Some(54321),
            None,
            Some(10.0),
            Some(U1::V1(20.0)),
            None,
        )
    }
    fn casts(
        a: Casts1,
        b: Casts2,
        c: Casts3,
        d: Casts4,
        e: Casts5,
        f: Casts6,
    ) -> (Casts1, Casts2, Casts3, Casts4, Casts5, Casts6) {
        match a {
            Casts1::A(-12345) => {}
            _ => panic!(),
        }
        match b {
            Casts2::B(x) => assert_eq!(x, 123.45),
            _ => panic!(),
        }
        match c {
            Casts3::B(12345) => {}
            _ => panic!(),
        }
        match d {
            Casts4::B(-12345) => {}
            _ => panic!(),
        }
        match e {
            Casts5::A(x) => assert_eq!(x, -123.45),
            _ => panic!(),
        }
        match f {
            Casts6::B((12345, 56789)) => {}
            _ => panic!(),
        }

        (
            Casts1::B(-123.45),
            Casts2::A(123.45),
            Casts3::A(12.345),
            Casts4::A(12345),
            Casts5::B(-12345),
            Casts6::A((1234.5, 56789)),
        )
    }
    fn expected_arg(
        a: Result<(), ()>,
        b: Result<(), E1>,
        c: Result<E1, ()>,
        d: Result<(), ()>,
        e: Result<u32, V1>,
        f: Result<String, Vec<u8>>,
    ) {
        assert_eq!(a, Ok(()));
        match b {
            Err(E1::A) => {}
            _ => panic!(),
        }
        match c {
            Ok(E1::A) => {}
            _ => panic!(),
        }
        assert_eq!(d, Err(()));
        match e {
            Ok(12345) => {}
            _ => panic!(),
        }
        match f {
            Ok(x) => assert_eq!(x, "I heart Wasm!"),
            _ => panic!(),
        }
    }
    fn expected_result() -> (
        Result<(), ()>,
        Result<(), E1>,
        Result<E1, ()>,
        Result<(), ()>,
        Result<u32, V1>,
        Result<String, Vec<u8>>,
    ) {
        (
            Err(()),
            Ok(()),
            Err(()),
            Ok(()),
            Err(V1::D("hi".to_string())),
            Err(vec![1, 2, 3, 4, 5, 6]),
        )
    }
}
