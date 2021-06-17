witx_bindgen_rust::import!("crates/lists/lists.witx");

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
                a0: 1,
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
                a0: 12,
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
    assert_eq!(x[0].a0, 1);
    assert_eq!(x[0].a1, 5);
    assert_eq!(x[0].a2, 2);
    assert_eq!(x[0].a3, 7);
    assert_eq!(x[0].a4, 11);
    assert_eq!(x[0].b, "hello!");
    assert_eq!(x[0].c, &[1, 2, 3, 4, 5]);
}
