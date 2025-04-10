include!(env!("BINDINGS"));

use test::options::to_test::*;

fn main() {
    option_none_param(None);
    option_some_param(Some("foo"));
    assert!(option_none_result().is_none());
    assert_eq!(option_some_result(), Some("foo".to_string()));
    assert_eq!(option_roundtrip(Some("foo")), Some("foo".to_string()));
    assert_eq!(double_option_roundtrip(Some(Some(42))), Some(Some(42)));
    assert_eq!(double_option_roundtrip(Some(None)), Some(None));
    assert_eq!(double_option_roundtrip(None), None);
}
