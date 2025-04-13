include!(env!("BINDINGS"));

use test::results::test::*;

fn main() {
    assert_eq!(string_error(0.0), Err("zero".to_owned()));
    assert_eq!(string_error(1.0), Ok(1.0));

    assert_eq!(enum_error(0.0), Err(E::A));
    assert_eq!(enum_error(1.0), Ok(1.0));

    assert!(matches!(
        record_error(0.0),
        Err(E2 {
            line: 420,
            column: 0
        })
    ));
    assert!(matches!(
        record_error(1.0),
        Err(E2 {
            line: 77,
            column: 2
        })
    ));
    assert!(record_error(2.0).is_ok());

    assert!(matches!(
        variant_error(0.0),
        Err(E3::E2(E2 {
            line: 420,
            column: 0
        }))
    ));
    assert!(matches!(variant_error(1.0), Err(E3::E1(E::B))));
    assert!(matches!(variant_error(2.0), Err(E3::E1(E::C))));

    assert_eq!(empty_error(0), Err(()));
    assert_eq!(empty_error(1), Ok(42));
    assert_eq!(empty_error(2), Ok(2));

    assert_eq!(double_error(0), Ok(Ok(())));
    assert_eq!(double_error(1), Ok(Err("one".into())));
    assert_eq!(double_error(2), Err("two".into()));
}
