// TODO: this test fails to compile with `--raw-strings`, that should be fixed.

include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::cat::Guest for Test {
    fn foo(x: String) {
        assert_eq!(x, "hello");
    }

    fn bar() -> String {
        "world".into()
    }
}
