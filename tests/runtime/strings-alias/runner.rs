include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // Test the argument is `&str`
        cat::foo("hello");

        // Test the return type is `String`
        let t: String = cat::bar();
        assert_eq!(t, "world");
    }
}
