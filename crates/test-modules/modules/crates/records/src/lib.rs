witx_bindgen_rust::export!("../../../tests/records.witx");

use records::*;

struct Component;

impl Records for Component {
    fn tuple_arg(&self, x: (char, u32)) {
        assert_eq!(x.0, 'a');
        assert_eq!(x.1, 0);
    }
    fn tuple_result(&self) -> (char, u32) {
        ('b', 1)
    }
    fn empty_arg(&self, _: Empty) {}
    fn empty_result(&self) -> Empty {
        Empty {}
    }
    fn scalar_arg(&self, x: Scalars) {
        assert_eq!(x.a, 1);
        assert_eq!(x.b, 2);
    }
    fn scalar_result(&self) -> Scalars {
        Scalars { a: 3, b: 4 }
    }
    fn flags_arg(&self, x: ReallyFlags) {
        assert_eq!(
            x,
            REALLY_FLAGS_B | REALLY_FLAGS_E | REALLY_FLAGS_F | REALLY_FLAGS_G | REALLY_FLAGS_I
        );
    }
    fn flags_result(&self) -> ReallyFlags {
        REALLY_FLAGS_A | REALLY_FLAGS_C | REALLY_FLAGS_D | REALLY_FLAGS_H
    }
    fn aggregate_arg(&self, x: Aggregates) {
        assert_eq!(x.a.a, 10);
        assert_eq!(x.a.b, 100);
        assert_eq!(x.b, 7);
        assert_eq!(x.d, "hello world!");
        assert_eq!(x.e, REALLY_FLAGS_F);
    }
    fn aggregate_result(&self) -> Aggregates {
        Aggregates {
            a: Scalars { a: 11, b: 101 },
            b: 8,
            c: Empty {},
            d: "I love Wasm!".to_string(),
            e: REALLY_FLAGS_G,
        }
    }
}

fn records() -> &'static impl Records {
    static INSTANCE: Component = Component;
    &INSTANCE
}
