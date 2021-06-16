witx_bindgen_rust::import!("crates/records/records.witx");

use records::{AggregatesParam, Empty, Scalars};

fn main() {
    records::tuple_arg(('a', 0));

    assert_eq!(records::tuple_result(), ('b', 1));

    records::empty_arg(Empty {});

    records::empty_result();

    records::scalar_arg(Scalars { a: 1, b: 2 });

    let x = records::scalar_result();
    assert_eq!(x.a, 3);
    assert_eq!(x.b, 4);

    records::aggregate_arg(AggregatesParam {
        a: Scalars { a: 10, b: 100 },
        b: 7,
        c: Empty {},
        d: "hello world!",
    });

    let x = records::aggregate_result();
    assert_eq!(x.a.a, 11);
    assert_eq!(x.a.b, 101);
    assert_eq!(x.b, 8);
    assert_eq!(x.d, "I love Wasm!");
}
