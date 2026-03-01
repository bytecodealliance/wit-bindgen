include!(env!("BINDINGS"));

use crate::test::list_in_variant::to_test::*;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // list-in-option (Bug 1: list freed inside match arm before FFI call)
        let hw: Vec<String> = ["hello", "world"].into_iter().map(Into::into).collect();
        assert_eq!(list_in_option(Some(&hw)), "hello,world");
        assert_eq!(list_in_option(None), "none");

        // list-in-variant (Bug 1: same pattern with variant)
        let fbb = PayloadOrEmpty::WithData(vec!["foo".into(), "bar".into(), "baz".into()]);
        assert_eq!(list_in_variant(&fbb), "foo,bar,baz");
        assert_eq!(list_in_variant(&PayloadOrEmpty::Empty), "empty");

        // list-in-result (Bug 1: same pattern with result)
        let abc: Vec<String> = ["a", "b", "c"].into_iter().map(Into::into).collect();
        assert_eq!(list_in_result(Ok(&abc)), "a,b,c");
        assert_eq!(list_in_result(Err("oops")), "err:oops");

        // list-in-option-with-return (Bug 1 + Bug 2: freed list + return_area read-after-free)
        let hw2: Vec<String> = ["hello", "world"].into_iter().map(Into::into).collect();
        let s = list_in_option_with_return(Some(&hw2));
        assert_eq!(s.count, 2);
        assert_eq!(s.label, "hello,world");
        let s = list_in_option_with_return(None);
        assert_eq!(s.count, 0);
        assert_eq!(s.label, "none");

        // top-level-list (NOT affected — contrast case)
        let xyz: Vec<String> = ["x", "y", "z"].into_iter().map(Into::into).collect();
        assert_eq!(top_level_list(&xyz), "x,y,z");
    }
}
