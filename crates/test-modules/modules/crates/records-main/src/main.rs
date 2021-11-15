wai_bindgen_rust::import!("crates/records/records.wai");

use records::*;

fn main() {
    tuple_arg(('a', 0));

    assert_eq!(tuple_result(), ('b', 1));

    empty_arg(Empty {});

    empty_result();

    scalar_arg(Scalars { a: 1, b: 2 });

    let x = scalar_result();
    assert_eq!(x.a, 3);
    assert_eq!(x.b, 4);

    flags_arg(REALLY_FLAGS_B | REALLY_FLAGS_E | REALLY_FLAGS_F | REALLY_FLAGS_G | REALLY_FLAGS_I);
    let x = flags_result();
    assert_eq!(
        x,
        REALLY_FLAGS_A | REALLY_FLAGS_C | REALLY_FLAGS_D | REALLY_FLAGS_H
    );

    aggregate_arg(AggregatesParam {
        a: Scalars { a: 10, b: 100 },
        b: 7,
        c: Empty {},
        d: "hello world!",
        e: REALLY_FLAGS_F,
    });

    let x = aggregate_result();
    assert_eq!(x.a.a, 11);
    assert_eq!(x.a.b, 101);
    assert_eq!(x.b, 8);
    assert_eq!(x.d, "I love Wasm!");
    assert_eq!(x.e, REALLY_FLAGS_G);
}
