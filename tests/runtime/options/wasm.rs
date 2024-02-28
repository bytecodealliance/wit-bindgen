wit_bindgen::generate!({
  path: "../../tests/runtime/options",
});

struct Component;

export!(Component);

impl Guest for Component {
    fn test_imports() {
        use test::options::test::*;

        option_none_param(None);
        option_some_param(Some("foo"));
        assert!(option_none_result().is_none());
        assert_eq!(option_some_result(), Some("foo".to_string()));
        assert_eq!(option_roundtrip(Some("foo")), Some("foo".to_string()));
    }
}

impl exports::test::options::test::Guest for Component {
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
}
