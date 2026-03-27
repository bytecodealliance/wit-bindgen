include!(env!("BINDINGS"));
struct Component;

export!(Component);

impl exports::test::options::to_test::Guest for Component {
    fn option_none_param(a: Option<String>) {
        assert!(a.is_none());
    }

    fn option_none_result() -> Option<String> {
        None
    }

    fn option_some_param(a: Option<String>) {
        assert_eq!(a, Some("foo".to_string()));
    }

    fn option_some_result() -> Option<String> {
        Some("foo".to_string())
    }

    fn option_roundtrip(a: Option<String>) -> Option<String> {
        a
    }

    fn double_option_roundtrip(a: Option<Option<u32>>) -> Option<Option<u32>> {
        a
    }
}
