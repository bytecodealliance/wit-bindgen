include!(env!("BINDINGS"));

use crate::test::flavorful::to_test::*;

#[path = "../lists/alloc.rs"]
mod alloc;

fn main() {
    let before = alloc::get();
    run();
    assert_eq!(before, alloc::get());
}

fn run() {
    f_list_in_record1(&ListInRecord1 {
        a: "list_in_record1".to_string(),
    });
    assert_eq!(f_list_in_record2().a, "list_in_record2");

    assert_eq!(
        f_list_in_record3(&ListInRecord3 {
            a: "list_in_record3 input".to_string()
        })
        .a,
        "list_in_record3 output"
    );

    assert_eq!(
        f_list_in_record4(&ListInAlias {
            a: "input4".to_string()
        })
        .a,
        "result4"
    );

    f_list_in_variant1(&Some("foo".to_string()), &Err("bar".to_string()));
    assert_eq!(f_list_in_variant2(), Some("list_in_variant2".to_string()));
    assert_eq!(
        f_list_in_variant3(&Some("input3".to_string())),
        Some("output3".to_string())
    );

    assert!(errno_result().is_err());
    MyErrno::A.to_string();
    _ = format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    assert!(errno_result().is_ok());

    let (a, b) = list_typedefs(&"typedef1".to_string(), &vec!["typedef2".to_string()]);
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");

    let (a, b, c) = list_of_variants(
        &[true, false],
        &[Ok(()), Err(())],
        &[MyErrno::Success, MyErrno::A],
    );
    assert_eq!(a, [false, true]);
    assert_eq!(b, [Err(()), Ok(())]);
    assert_eq!(c, [MyErrno::A, MyErrno::B]);
}
