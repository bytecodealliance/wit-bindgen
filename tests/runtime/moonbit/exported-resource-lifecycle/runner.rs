include!(env!("BINDINGS"));

use test::exported_resource_lifecycle::api::*;

struct Component;
export!(Component);

impl Guest for Component {
    fn run() {
        // Fallible constructors register only successful application objects.
        assert_eq!(Tracked::new(0, false).unwrap_err(), "zero");
        assert!(take_dropped().is_empty());
        drop(Tracked::new(1, false).unwrap());
        assert_eq!(take_dropped(), [1]);

        // Transferring ownership invalidates every alias to the outgoing wrapper.
        let value = retain_alias(Tracked::new(2, false).unwrap());
        drop_saved_alias();
        assert!(take_dropped().is_empty());
        drop(value);
        assert_eq!(take_dropped(), [2]);

        // Dropping a locally owned wrapper also invalidates its aliases.
        drop_twice(Tracked::new(3, false).unwrap());
        assert_eq!(take_dropped(), [3]);

        // Destruction may reuse its released slot. The child must remain usable
        // after the outer destructor finishes, with its own object still stored.
        drop(Tracked::new(4, true).unwrap());
        assert_eq!(take_dropped(), [4]);
        drop(take_reentrant_child());
        assert_eq!(take_dropped(), [5]);
    }
}
