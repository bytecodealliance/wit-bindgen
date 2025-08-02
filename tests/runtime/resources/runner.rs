include!(env!("BINDINGS"));

use crate::exports::*;

fn main() {
    test_imports().unwrap();

    let x = X::new(5);
    assert_eq!(x.get_a(), 5);
    x.set_a(10);
    assert_eq!(x.get_a(), 10);
    let z1 = Z::new(10);
    assert_eq!(z1.get_a(), 10);
    let z2 = Z::new(20);
    assert_eq!(z2.get_a(), 20);

    let xadd = X::add(x, 5);
    assert_eq!(xadd.get_a(), 15);

    let zadd = add(&z1, &z2);
    assert_eq!(zadd.get_a(), 30);

    let dropped_zs_start = Z::num_dropped();

    drop(z1);
    drop(z2);

    consume(xadd);

    let dropped_zs_end = Z::num_dropped();
    if dropped_zs_start != 0 {
        assert_eq!(dropped_zs_end, dropped_zs_start + 2);
    }
}
