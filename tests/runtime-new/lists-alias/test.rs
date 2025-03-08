include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::cat::Guest for Test {
    fn foo(x: Vec<u8>) {
        assert_eq!(x, b"hello");
    }

    fn bar() -> Vec<u8> {
        b"world".into()
    }
}
