wit_bindgen_rust::import!("../types/types.wit");

fn main() {
    types::a();
    assert_eq!(
        types::b(1, -2, 3, 4, 5, -6, 7, 8),
        (1, -2, 3, 4, 5, -6, 7, 8)
    );
    assert_eq!(types::c(1.7, 2.6), (1.7, 2.6));
    assert_eq!(types::d("this is a string"), "this is a string");
    assert_eq!(types::e(), "hello world!");

    let (a, b, c) = types::f();
    assert_eq!((a, b.as_str(), c), (13, "hi", 37));

    assert_eq!(types::g(101), 101);
}
