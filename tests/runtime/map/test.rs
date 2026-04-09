include!(env!("BINDINGS"));

use crate::exports::test::maps::to_test::{
    BytesByName, IdsByName, LabeledEntry, MapOrString, NamesById,
};

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

    fn multi_param_roundtrip(a: NamesById, b: BytesByName) -> (IdsByName, BytesByName) {
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 1);
        let mut ids = IdsByName::new();
        for (id, name) in a {
            ids.insert(name, id);
        }
        (ids, b)
    }

    fn nested_roundtrip(
        a: wit_bindgen::rt::Map<String, wit_bindgen::rt::Map<u32, String>>,
    ) -> wit_bindgen::rt::Map<String, wit_bindgen::rt::Map<u32, String>> {
        assert_eq!(a.len(), 2);
        let inner = a.get("group-a").unwrap();
        assert_eq!(inner.get(&1).map(String::as_str), Some("one"));
        assert_eq!(inner.get(&2).map(String::as_str), Some("two"));
        let inner2 = a.get("group-b").unwrap();
        assert_eq!(inner2.get(&10).map(String::as_str), Some("ten"));
        a
    }

    fn variant_roundtrip(a: MapOrString) -> MapOrString {
        a
    }

    fn result_roundtrip(a: Result<NamesById, String>) -> Result<NamesById, String> {
        a
    }

    fn tuple_roundtrip(a: (NamesById, u64)) -> (NamesById, u64) {
        assert_eq!(a.0.len(), 1);
        assert_eq!(a.0.get(&7).map(String::as_str), Some("seven"));
        assert_eq!(a.1, 42);
        a
    }

    fn single_entry_roundtrip(a: NamesById) -> NamesById {
        assert_eq!(a.len(), 1);
        a
    }
}
