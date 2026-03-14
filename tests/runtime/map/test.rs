include!(env!("BINDINGS"));

use crate::exports::test::maps::to_test::{BytesByName, IdsByName, NamesById};

struct Component;

export!(Component);

impl exports::test::maps::to_test::Guest for Component {
    fn named_roundtrip(a: NamesById) -> IdsByName {
        assert_eq!(a.get(&1).map(String::as_str), Some("uno"));
        assert_eq!(a.get(&2).map(String::as_str), Some("two"));

        let mut result = IdsByName::new();
        for (id, name) in a {
            result.insert(name, id);
        }
        result
    }

    fn bytes_roundtrip(a: BytesByName) -> BytesByName {
        assert_eq!(a.get("hello").map(Vec::as_slice), Some(b"world".as_slice()));
        assert_eq!(a.get("bin").map(Vec::as_slice), Some([0u8, 1, 2].as_slice()));
        a
    }
}
