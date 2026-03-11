//@ wasmtime-flags = '-Wcomponent-model-map'

include!(env!("BINDINGS"));

use test::maps::to_test::*;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let ids_by_name = named_roundtrip(&[
            (1, "one".to_string()),
            (1, "uno".to_string()),
            (2, "two".to_string()),
        ]);
        assert_eq!(ids_by_name.get("uno"), Some(&1));
        assert_eq!(ids_by_name.get("two"), Some(&2));
        assert_eq!(ids_by_name.get("one"), None);

        let bytes_by_name = bytes_roundtrip(&[
            ("hello".to_string(), b"world".to_vec()),
            ("bin".to_string(), vec![0u8, 1, 2]),
        ]);
        assert_eq!(
            bytes_by_name.get("hello").map(Vec::as_slice),
            Some(b"world".as_slice())
        );
        assert_eq!(
            bytes_by_name.get("bin").map(Vec::as_slice),
            Some([0u8, 1, 2].as_slice())
        );
    }
}
