include!(env!("BINDINGS"));

use test::lists::to_test::*;

struct Guard {
    me_before: usize,
    remote_before: u32,
}

impl Guard {
    fn new() -> Guard {
        Guard {
            me_before: alloc::get(),
            remote_before: allocated_bytes(),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        assert_eq!(self.me_before, alloc::get());
        assert_eq!(self.remote_before, allocated_bytes());
    }
}

mod alloc;

fn main() {
    let _guard_over_entire_function = Guard::new();

    {
        let _guard = Guard::new();
        empty_list_param(&[]);
    }
    {
        let _guard = Guard::new();
        empty_string_param("");
    }
    {
        let _guard = Guard::new();
        assert!(empty_list_result().is_empty());
    }
    {
        let _guard = Guard::new();
        assert!(empty_string_result().is_empty());
    }

    {
        let _guard = Guard::new();
        list_param(&[1, 2, 3, 4]);
    }
    {
        let _guard = Guard::new();
        list_param2("foo");
    }
    {
        let _guard = Guard::new();
        list_param3(&["foo".to_owned(), "bar".to_owned(), "baz".to_owned()]);
    }
    {
        let _guard = Guard::new();
        list_param4(&[
            vec!["foo".to_owned(), "bar".to_owned()],
            vec!["baz".to_owned()],
        ]);
    }
    {
        let _guard = Guard::new();
        list_param5(&[(1, 2, 3), (4, 5, 6)]);
    }
    {
        let _guard = Guard::new();
        let large_list: Vec<String> = (0..1000).map(|_| "string".to_string()).collect();
        list_param_large(&large_list);
    }
    {
        let _guard = Guard::new();
        assert_eq!(list_result(), [1, 2, 3, 4, 5]);
    }
    {
        let _guard = Guard::new();
        assert_eq!(list_result2(), "hello!");
    }
    {
        let _guard = Guard::new();
        assert_eq!(list_result3(), ["hello,", "world!"]);
    }

    {
        let _guard = Guard::new();
        assert_eq!(list_roundtrip(&[]), []);
    }
    {
        let _guard = Guard::new();
        assert_eq!(list_roundtrip(b"x"), b"x");
    }
    {
        let _guard = Guard::new();
        assert_eq!(list_roundtrip(b"hello"), b"hello");
    }

    {
        let _guard = Guard::new();
        assert_eq!(string_roundtrip("x"), "x");
    }
    {
        let _guard = Guard::new();
        assert_eq!(string_roundtrip(""), "");
    }
    {
        let _guard = Guard::new();
        assert_eq!(string_roundtrip("hello"), "hello");
    }
    {
        let _guard = Guard::new();
        assert_eq!(string_roundtrip("hello ⚑ world"), "hello ⚑ world");
    }

    {
        let _guard = Guard::new();
        assert_eq!(
            list_minmax8(&[u8::MIN, u8::MAX], &[i8::MIN, i8::MAX]),
            (vec![u8::MIN, u8::MAX], vec![i8::MIN, i8::MAX]),
        );
    }
    {
        let _guard = Guard::new();
        assert_eq!(
            list_minmax16(&[u16::MIN, u16::MAX], &[i16::MIN, i16::MAX]),
            (vec![u16::MIN, u16::MAX], vec![i16::MIN, i16::MAX]),
        );
    }
    {
        let _guard = Guard::new();
        assert_eq!(
            list_minmax32(&[u32::MIN, u32::MAX], &[i32::MIN, i32::MAX]),
            (vec![u32::MIN, u32::MAX], vec![i32::MIN, i32::MAX]),
        );
    }
    {
        let _guard = Guard::new();
        assert_eq!(
            list_minmax64(&[u64::MIN, u64::MAX], &[i64::MIN, i64::MAX]),
            (vec![u64::MIN, u64::MAX], vec![i64::MIN, i64::MAX]),
        );
    }
    {
        let _guard = Guard::new();
        assert_eq!(
            list_minmax_float(
                &[f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
                &[f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]
            ),
            (
                vec![f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
                vec![f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY],
            ),
        );
    }
}
