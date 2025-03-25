include!(env!("BINDINGS"));

fn main() {
    use test::dep0_1_0::test as v1;
    assert_eq!(v1::x(), 1.0);
    assert_eq!(v1::y(1.0), 2.0);

    use test::dep0_2_0::test as v2;
    assert_eq!(v2::x(), 2.0);
    assert_eq!(v2::z(1.0, 1.0), 4.0);
}
