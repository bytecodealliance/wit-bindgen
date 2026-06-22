//@ wasmtime-flags = '-Wcomponent-model-implements'

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        // Each labeled import is its own instance of `store`, so values set
        // through `primary` are independent from those set through `backup`.
        primary::set("key", "from-primary");
        backup::set("key", "from-backup");

        assert_eq!(primary::get("key").as_deref(), Some("from-primary"));
        assert_eq!(backup::get("key").as_deref(), Some("from-backup"));

        assert_eq!(primary::get("missing"), None);
        assert_eq!(backup::get("missing"), None);
    }
}
