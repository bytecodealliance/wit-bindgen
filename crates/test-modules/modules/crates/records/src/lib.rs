wit_bindgen_rust::export!("crates/records/records.wit");

use records::*;

struct Records;

impl records::Records for Records {
    fn tuple_arg(x: (char, u32)) {
        assert_eq!(x.0, 'a');
        assert_eq!(x.1, 0);
    }
    fn tuple_result() -> (char, u32) {
        ('b', 1)
    }
    fn empty_arg(_: Empty) {}
    fn empty_result() -> Empty {
        Empty {}
    }
    fn scalar_arg(x: Scalars) {
        assert_eq!(x.a, 1);
        assert_eq!(x.b, 2);
    }
    fn scalar_result() -> Scalars {
        Scalars { a: 3, b: 4 }
    }
    fn flags_arg(x: ReallyFlags) {
        assert_eq!(
            x,
            ReallyFlags::B | ReallyFlags::E | ReallyFlags::F | ReallyFlags::G | ReallyFlags::I
        );
    }
    fn flags_result() -> ReallyFlags {
        ReallyFlags::A | ReallyFlags::C | ReallyFlags::D | ReallyFlags::H
    }
    fn aggregate_arg(x: Aggregates) {
        assert_eq!(x.a.a, 10);
        assert_eq!(x.a.b, 100);
        assert_eq!(x.b, 7);
        assert_eq!(x.d, "hello world!");
        assert_eq!(x.e, ReallyFlags::F);
    }
    fn aggregate_result() -> Aggregates {
        Aggregates {
            a: Scalars { a: 11, b: 101 },
            b: 8,
            c: Empty {},
            d: "I love Wasm!".to_string(),
            e: ReallyFlags::G,
        }
    }
}
