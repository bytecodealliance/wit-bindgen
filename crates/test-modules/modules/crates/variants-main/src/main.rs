wit_bindgen_rust::import!("../variants/variants.wit");

use variants::*;

fn main() {
    e1_arg(E1::A);
    match e1_result() {
        E1::A => {}
    }

    u1_arg(U1::V0(1234));
    match u1_result() {
        U1::V1(x) => assert_eq!(x, 432.1),
        _ => panic!(),
    }

    v1_arg(V1Param::D("hello world!"));
    match v1_result() {
        V1Result::G(x) => assert_eq!(x, 54321),
        _ => panic!(),
    }

    bool_arg(true);
    assert!(!bool_result());

    option_arg(
        None,
        Some(()),
        Some(12345),
        Some(E1::A),
        None,
        Some(U1::V0(67890)),
        Some(Some(true)),
    );
    let x = option_result();
    assert_eq!(x.0, Some(false));
    assert_eq!(x.1, None);
    assert_eq!(x.2, Some(54321));
    match x.3 {
        None => {}
        _ => panic!(),
    }
    assert_eq!(x.4, Some(10.0));
    match x.5 {
        Some(U1::V1(x)) => assert_eq!(x, 20.0),
        _ => panic!(),
    }
    assert_eq!(x.6, None);

    let (a, b, c, d, e, f) = casts(
        Casts1::A(-12345),
        Casts2::B(123.45),
        Casts3::B(12345),
        Casts4::B(-12345),
        Casts5::A(-123.45),
        Casts6::B((12345, 56789)),
    );
    match a {
        Casts1::B(x) => assert_eq!(x, -123.45),
        _ => panic!(),
    }
    match b {
        Casts2::A(x) => assert_eq!(x, 123.45),
        _ => panic!(),
    }
    match c {
        Casts3::A(x) => assert_eq!(x, 12.345),
        _ => panic!(),
    }
    match d {
        Casts4::A(12345) => {}
        _ => panic!(),
    }
    match e {
        Casts5::B(-12345) => {}
        _ => panic!(),
    }
    match f {
        Casts6::A(x) => assert_eq!(x, ((1234.5, 56789))),
        _ => panic!(),
    }

    expected_arg(
        Ok(()),
        Err(E1::A),
        Ok(E1::A),
        Err(()),
        Ok(12345),
        Ok("I heart Wasm!"),
    );
    let (a, b, c, d, e, f) = expected_result();
    assert_eq!(a, Err(()));
    match b {
        Ok(()) => {}
        _ => panic!(),
    }
    match c {
        Err(()) => {}
        _ => panic!(),
    }
    assert_eq!(d, Ok(()));
    match e {
        Err(V1Result::D(x)) => assert_eq!(x, "hi"),
        _ => panic!(),
    }
    match f {
        Err(x) => assert_eq!(x, &[1, 2, 3, 4, 5, 6]),
        _ => panic!(),
    }
}
