include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // Startup may have run the constructors at instantiation or at the
        // first export call. Neither export call should run them again.
        assert_eq!(the::test::i::ctor_runs(), 1);
        assert_eq!(the::test::i::ctor_runs(), 1);
    }
}
