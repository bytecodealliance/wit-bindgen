wit_bindgen_rust::import!("crates/lists/lists.wit");

use lists::*;

fn main() {
    list_u8_param(&[5, 4, 3, 2, 1]);
    list_u16_param(&[10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    list_u32_param(&[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    list_u64_param(&[
        20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
    ]);
    list_s8_param(&[-1, 2, -3, 4, -5]);
    list_s16_param(&[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10]);
    list_s32_param(&[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15]);
    list_s64_param(&[
        -1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
    ]);
    list_f32_param(&[-1.1, 2.2, -3.3, 4.4, -5.5]);
    list_f64_param(&[-1.1, 2.2, -3.3, 4.4, -5.5]);

    assert_eq!(list_u8_ret(), &[5, 4, 3, 2, 1]);
    assert_eq!(list_u16_ret(), &[10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    assert_eq!(
        list_u32_ret(),
        &[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]
    );
    assert_eq!(
        list_u64_ret(),
        &[20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]
    );
    assert_eq!(list_s8_ret(), &[-1, 2, -3, 4, -5]);
    assert_eq!(list_s16_ret(), &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10]);
    assert_eq!(
        list_s32_ret(),
        &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15]
    );
    assert_eq!(
        list_s64_ret(),
        &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20]
    );
    assert_eq!(list_f32_ret(), &[1.1, -2.2, 3.3, -4.4, 5.5]);
    assert_eq!(list_f64_ret(), &[1.1, -2.2, 3.3, -4.4, 5.5]);

    assert_eq!(
        tuple_list(&[
            (1, -2),
            (3, 4),
            (5, -6),
            (7, 8),
            (9, -10),
            (11, 12),
            (13, -14)
        ]),
        &[
            (-1, 2),
            (3, 4),
            (-5, 6),
            (7, 8),
            (-9, 10),
            (11, 12),
            (-13, 14)
        ]
    );

    let x = tuple_string_list(&[(0, "hello"), (1, "world")]);
    assert_eq!(x.len(), 2);
    assert_eq!(x[0].0, "world");
    assert_eq!(x[0].1, 3);
    assert_eq!(x[1].0, "hello");
    assert_eq!(x[1].1, 4);

    let x = string_list(&["hello", "world"]);
    assert_eq!(x.len(), 4);
    assert_eq!(x[0], "I");
    assert_eq!(x[1], "love");
    assert_eq!(x[2], "Wasm");
    assert_eq!(x[3], "!");

    let x = record_list(&[
        SomeRecord {
            x: "guten tag!",
            y: OtherRecordParam {
                a1: 2,
                a2: 3,
                a3: 4,
                a4: 5,
                b: "6",
                c: &[7],
            },
            c1: 8,
            c2: 9,
            c3: 10,
            c4: 11,
        },
        SomeRecord {
            x: "guten morgen!",
            y: OtherRecordParam {
                a1: 13,
                a2: 14,
                a3: 15,
                a4: 16,
                b: "17",
                c: &[18, 19, 20],
            },
            c1: 21,
            c2: 22,
            c3: 23,
            c4: 24,
        },
    ]);

    assert_eq!(x.len(), 1);
    assert_eq!(x[0].a1, 5);
    assert_eq!(x[0].a2, 2);
    assert_eq!(x[0].a3, 7);
    assert_eq!(x[0].a4, 11);
    assert_eq!(x[0].b, "hello!");
    assert_eq!(x[0].c, &[1, 2, 3, 4, 5]);

    let x = variant_list(&[
        SomeVariant::B,
        SomeVariant::A("first"),
        SomeVariant::C(1244),
        SomeVariant::A("second"),
        SomeVariant::D(&[
            OtherVariantParam::B(4321),
            OtherVariantParam::A,
            OtherVariantParam::C("third"),
        ]),
    ]);

    assert_eq!(x.len(), 3);
    match &x[0] {
        OtherVariantResult::C(x) => assert_eq!(x, "a string"),
        _ => panic!(),
    }
    match &x[1] {
        OtherVariantResult::A => {}
        _ => panic!(),
    }
    match &x[2] {
        OtherVariantResult::B(332211) => {}
        _ => panic!(),
    }

    let x = load_store_everything(&[
        ("I", 0, 1, 2, 3, 4, 5, 6, 7, 7.1, 7.2, 'a'),
        ("love", 8, 9, 10, 11, 12, 13, 14, 15, 15.1, 15.2, 'b'),
        ("wasm", 16, 17, 18, 19, 20, 21, 22, 23, 23.1, 23.2, 'c'),
    ]);

    assert_eq!(x.len(), 3);
    assert_eq!(x[0].0, "I");
    assert_eq!(x[0].1, 0);
    assert_eq!(x[0].2, 1);
    assert_eq!(x[0].3, 2);
    assert_eq!(x[0].4, 3);
    assert_eq!(x[0].5, 4);
    assert_eq!(x[0].6, 5);
    assert_eq!(x[0].7, 6);
    assert_eq!(x[0].8, 7);
    assert_eq!(x[0].9, 7.1);
    assert_eq!(x[0].10, 7.2);
    assert_eq!(x[0].11, 'a');
    assert_eq!(x[1].0, "love");
    assert_eq!(x[1].1, 8);
    assert_eq!(x[1].2, 9);
    assert_eq!(x[1].3, 10);
    assert_eq!(x[1].4, 11);
    assert_eq!(x[1].5, 12);
    assert_eq!(x[1].6, 13);
    assert_eq!(x[1].7, 14);
    assert_eq!(x[1].8, 15);
    assert_eq!(x[1].9, 15.1);
    assert_eq!(x[1].10, 15.2);
    assert_eq!(x[1].11, 'b');
    assert_eq!(x[2].0, "wasm");
    assert_eq!(x[2].1, 16);
    assert_eq!(x[2].2, 17);
    assert_eq!(x[2].3, 18);
    assert_eq!(x[2].4, 19);
    assert_eq!(x[2].5, 20);
    assert_eq!(x[2].6, 21);
    assert_eq!(x[2].7, 22);
    assert_eq!(x[2].8, 23);
    assert_eq!(x[2].9, 23.1);
    assert_eq!(x[2].10, 23.2);
    assert_eq!(x[2].11, 'c');
}
