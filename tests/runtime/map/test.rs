include!(env!("BINDINGS"));

use crate::exports::test::maps::to_test::{BytesByName, IdsByName, LabeledEntry, NamesById};

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
        assert_eq!(
            a.get("hello").map(Vec::as_slice),
            Some(b"world".as_slice())
        );
        assert_eq!(
            a.get("bin").map(Vec::as_slice),
            Some([0u8, 1, 2].as_slice())
        );
        a
    }

    fn empty_roundtrip(a: NamesById) -> NamesById {
        assert!(a.is_empty());
        a
    }

    fn option_roundtrip(
        a: wit_bindgen::rt::Map<String, Option<u32>>,
    ) -> wit_bindgen::rt::Map<String, Option<u32>> {
        assert_eq!(a.get("some"), Some(&Some(42)));
        assert_eq!(a.get("none"), Some(&None));
        a
    }

    fn record_roundtrip(a: LabeledEntry) -> LabeledEntry {
        assert_eq!(a.label, "test-label");
        assert_eq!(a.values.len(), 2);
        assert_eq!(a.values.get(&10).map(String::as_str), Some("ten"));
        assert_eq!(a.values.get(&20).map(String::as_str), Some("twenty"));
        a
    }

    fn inline_roundtrip(
        a: wit_bindgen::rt::Map<u32, String>,
    ) -> wit_bindgen::rt::Map<String, u32> {
        let mut result = wit_bindgen::rt::Map::new();
        for (k, v) in a {
            result.insert(v, k);
        }
        result
    }

    fn large_roundtrip(a: NamesById) -> NamesById {
        a
    }
}
