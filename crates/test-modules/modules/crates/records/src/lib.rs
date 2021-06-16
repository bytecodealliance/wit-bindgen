witx_bindgen_rust::export!("crates/records/records.witx");

use records::{Aggregates, Empty, Scalars};

struct Component;

impl records::Records for Component {
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
    fn aggregate_arg(&self, x: Aggregates) {
        assert_eq!(x.a.a, 10);
        assert_eq!(x.a.b, 100);
        assert_eq!(x.b, 7);
        assert_eq!(x.d, "hello world!");
    }
    fn aggregate_result(&self) -> Aggregates {
        Aggregates {
            a: Scalars { a: 11, b: 101 },
            b: 8,
            c: Empty {},
            d: "I love Wasm!".to_string(),
        }
    }
}

fn records() -> &'static impl records::Records {
    static INSTANCE: Component = Component;
    &INSTANCE
}
