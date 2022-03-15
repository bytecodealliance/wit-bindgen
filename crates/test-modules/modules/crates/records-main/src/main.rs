wit_bindgen_rust::import!("../records/records.wit");

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

    flags_arg(ReallyFlags::B | ReallyFlags::E | ReallyFlags::F | ReallyFlags::G | ReallyFlags::I);
    let x = flags_result();
    assert_eq!(
        x,
        ReallyFlags::A | ReallyFlags::C | ReallyFlags::D | ReallyFlags::H
    );

    aggregate_arg(AggregatesParam {
        a: Scalars { a: 10, b: 100 },
        b: 7,
        c: Empty {},
        d: "hello world!",
        e: ReallyFlags::F,
    });

    let x = aggregate_result();
    assert_eq!(x.a.a, 11);
    assert_eq!(x.a.b, 101);
    assert_eq!(x.b, 8);
    assert_eq!(x.d, "I love Wasm!");
    assert_eq!(x.e, ReallyFlags::G);
}
