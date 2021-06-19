witx_bindgen_rust::export!("crates/lists/lists.witx");

use lists::*;

struct Component;

impl Lists for Component {
    fn list_u8_param(&self, x: Vec<u8>) {
        assert_eq!(x, &[5, 4, 3, 2, 1]);
    }
    fn list_u16_param(&self, x: Vec<u16>) {
        assert_eq!(x, &[10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }
    fn list_u32_param(&self, x: Vec<u32>) {
        assert_eq!(x, &[15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }
    fn list_u64_param(&self, x: Vec<u64>) {
        assert_eq!(
            x,
            &[20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]
        );
    }
    fn list_s8_param(&self, x: Vec<i8>) {
        assert_eq!(x, &[-1, 2, -3, 4, -5]);
    }
    fn list_s16_param(&self, x: Vec<i16>) {
        assert_eq!(x, &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10]);
    }
    fn list_s32_param(&self, x: Vec<i32>) {
        assert_eq!(
            x,
            &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15]
        );
    }
    fn list_s64_param(&self, x: Vec<i64>) {
        assert_eq!(
            x,
            &[-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20]
        );
    }
    fn list_u8_ret(&self) -> Vec<u8> {
        vec![5, 4, 3, 2, 1]
    }
    fn list_u16_ret(&self) -> Vec<u16> {
        vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]
    }
    fn list_u32_ret(&self) -> Vec<u32> {
        vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]
    }
    fn list_u64_ret(&self) -> Vec<u64> {
        vec![
            20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
        ]
    }
    fn list_s8_ret(&self) -> Vec<i8> {
        vec![-1, 2, -3, 4, -5]
    }
    fn list_s16_ret(&self) -> Vec<i16> {
        vec![-1, 2, -3, 4, -5, 6, -7, 8, -9, 10]
    }
    fn list_s32_ret(&self) -> Vec<i32> {
        vec![-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15]
    }
    fn list_s64_ret(&self) -> Vec<i64> {
        vec![
            -1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20,
        ]
    }
    fn tuple_list(&self, x: Vec<(u8, i8)>) -> Vec<(i64, u32)> {
        assert_eq!(
            x,
            &[
                (1, -2),
                (3, 4),
                (5, -6),
                (7, 8),
                (9, -10),
                (11, 12),
                (13, -14)
            ]
        );
        vec![
            (-1, 2),
            (3, 4),
            (-5, 6),
            (7, 8),
            (-9, 10),
            (11, 12),
            (-13, 14),
        ]
    }
    fn tuple_string_list(&self, x: Vec<(u8, String)>) -> Vec<(String, u8)> {
        assert_eq!(x.len(), 2);
        assert_eq!(x[0].0, 0);
        assert_eq!(x[0].1, "hello");
        assert_eq!(x[1].0, 1);
        assert_eq!(x[1].1, "world");
        vec![("world".to_string(), 3), ("hello".to_string(), 4)]
    }
    fn string_list(&self, x: Vec<String>) -> Vec<String> {
        assert_eq!(x.len(), 2);
        assert_eq!(x[0], "hello");
        assert_eq!(x[1], "world");
        vec![
            "I".to_string(),
            "love".to_string(),
            "Wasm".to_string(),
            "!".to_string(),
        ]
    }
    fn record_list(&self, x: Vec<SomeRecord>) -> Vec<OtherRecord> {
        assert_eq!(x.len(), 2);
        assert_eq!(x[0].x, "guten tag!");
        assert_eq!(x[0].y.a0, 1);
        assert_eq!(x[0].y.a1, 2);
        assert_eq!(x[0].y.a2, 3);
        assert_eq!(x[0].y.a3, 4);
        assert_eq!(x[0].y.a4, 5);
        assert_eq!(x[0].y.b, "6");
        assert_eq!(x[0].y.c.len(), 1);
        assert_eq!(x[0].y.c[0], 7);
        assert_eq!(x[0].c1, 8);
        assert_eq!(x[0].c2, 9);
        assert_eq!(x[0].c3, 10);
        assert_eq!(x[0].c4, 11);
        assert_eq!(x[1].x, "guten morgen!");
        assert_eq!(x[1].y.a0, 12);
        assert_eq!(x[1].y.a1, 13);
        assert_eq!(x[1].y.a2, 14);
        assert_eq!(x[1].y.a3, 15);
        assert_eq!(x[1].y.a4, 16);
        assert_eq!(x[1].y.b, "17");
        assert_eq!(x[1].y.c.len(), 3);
        assert_eq!(x[1].y.c[0], 18);
        assert_eq!(x[1].y.c[1], 19);
        assert_eq!(x[1].y.c[2], 20);
        assert_eq!(x[1].c1, 21);
        assert_eq!(x[1].c2, 22);
        assert_eq!(x[1].c3, 23);
        assert_eq!(x[1].c4, 24);

        vec![OtherRecord {
            a0: 1,
            a1: 5,
            a2: 2,
            a3: 7,
            a4: 11,
            b: "hello!".to_string(),
            c: vec![1, 2, 3, 4, 5],
        }]
    }
}

fn lists() -> &'static impl Lists {
    static INSTANCE: Component = Component;
    &INSTANCE
}
