//@ wasmtime-flags = '-Wcomponent-model-accessors'

include!(env!("BINDINGS"));

use crate::test::accessors::i::*;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        assert_eq!(counter(), 0);
        set_counter(5);
        assert_eq!(counter(), 5);
        set_counter(u32::MAX);
        assert_eq!(counter(), u32::MAX);

        assert_eq!(read_only(), "read only");

        assert_eq!(bounded(), 0);
        assert_eq!(set_bounded(10), Ok(()));
        assert_eq!(bounded(), 10);
        assert_eq!(set_bounded(101), Err("101 is out of bounds".to_string()));
        assert_eq!(bounded(), 10);

        let a = Blob::new(&[1, 2, 3]);
        let b = Blob::new(&[4, 5]);
        assert_eq!(a.contents(), [1, 2, 3]);
        assert_eq!(a.position(), 0);
        assert_eq!(b.position(), 0);
        a.set_position(2);
        assert_eq!(a.position(), 2);
        assert_eq!(b.position(), 0);
        b.set_position(1);
        assert_eq!(a.position(), 2);
        assert_eq!(b.position(), 1);

        assert_eq!(a.label(), "");
        assert_eq!(a.set_label("hello"), Ok(()));
        assert_eq!(a.label(), "hello");
        assert_eq!(b.label(), "");
        assert_eq!(a.set_label(""), Err("label must not be empty".to_string()));
        assert_eq!(a.label(), "hello");

        assert_eq!(Blob::max_size(), 1024);
        assert_eq!(Blob::set_max_size(2048), Ok(()));
        assert_eq!(Blob::max_size(), 2048);
        assert_eq!(
            Blob::set_max_size(0),
            Err("max size must be nonzero".to_string())
        );
        assert_eq!(Blob::max_size(), 2048);
    }
}
