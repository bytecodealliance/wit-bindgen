include!(env!("BINDINGS"));

struct Component;

export!(Component);

mod alloc;

impl exports::test::lists::to_test::Guest for Component {
    fn allocated_bytes() -> u32 {
        alloc::get().try_into().unwrap()
    }

    fn empty_list_param(a: Vec<u8>) {
        assert!(a.is_empty());
    }

    fn empty_string_param(a: String) {
        assert!(a.is_empty());
    }

    fn empty_list_result() -> Vec<u8> {
        Vec::new()
    }

    fn empty_string_result() -> String {
        String::new()
    }

    fn list_param(list: Vec<u8>) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(ptr: String) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(ptr: Vec<String>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(ptr: Vec<Vec<String>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_param5(ptr: Vec<(u8, u32, u8)>) {
        assert_eq!(ptr, [(1, 2, 3), (4, 5, 6)]);
    }

    fn list_param_large(ptr: Vec<String>) {
        assert_eq!(ptr.len(), 1000);
    }

    fn list_result() -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2() -> String {
        "hello!".to_string()
    }

    fn list_result3() -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }

    fn list_roundtrip(x: Vec<u8>) -> Vec<u8> {
        x.clone()
    }

    fn string_roundtrip(x: String) -> String {
        x.clone()
    }

    fn list_minmax8(a: Vec<u8>, b: Vec<i8>) -> (Vec<u8>, Vec<i8>) {
        (a, b)
    }

    fn list_minmax16(a: Vec<u16>, b: Vec<i16>) -> (Vec<u16>, Vec<i16>) {
        (a, b)
    }

    fn list_minmax32(a: Vec<u32>, b: Vec<i32>) -> (Vec<u32>, Vec<i32>) {
        (a, b)
    }

    fn list_minmax64(a: Vec<u64>, b: Vec<i64>) -> (Vec<u64>, Vec<i64>) {
        (a, b)
    }

    fn list_minmax_float(a: Vec<f32>, b: Vec<f64>) -> (Vec<f32>, Vec<f64>) {
        (a, b)
    }
}
