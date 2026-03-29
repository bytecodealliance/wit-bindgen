//@ wasmtime-flags = '-Wcomponent-model-map'

include!(env!("BINDINGS"));

use test::maps::to_test::*;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        test_named_roundtrip();
        test_bytes_roundtrip();
        test_empty_roundtrip();
        test_option_roundtrip();
        test_record_roundtrip();
        test_inline_roundtrip();
        test_large_map();
    }
}

fn test_named_roundtrip() {
    let mut input = NamesById::new();
    input.insert(1, "one".to_string());
    input.insert(1, "uno".to_string());
    input.insert(2, "two".to_string());
    let ids_by_name = named_roundtrip(&input);
    assert_eq!(ids_by_name.get("uno"), Some(&1));
    assert_eq!(ids_by_name.get("two"), Some(&2));
    assert_eq!(ids_by_name.get("one"), None);
}

fn test_bytes_roundtrip() {
    let mut bytes_input = BytesByName::new();
    bytes_input.insert("hello".to_string(), b"world".to_vec());
    bytes_input.insert("bin".to_string(), vec![0u8, 1, 2]);
    let bytes_by_name = bytes_roundtrip(&bytes_input);
    assert_eq!(
        bytes_by_name.get("hello").map(Vec::as_slice),
        Some(b"world".as_slice())
    );
    assert_eq!(
        bytes_by_name.get("bin").map(Vec::as_slice),
        Some([0u8, 1, 2].as_slice())
    );
}

fn test_empty_roundtrip() {
    let empty = NamesById::new();
    let result = empty_roundtrip(&empty);
    assert!(result.is_empty());
}

fn test_option_roundtrip() {
    let mut input = wit_bindgen::rt::Map::new();
    input.insert("some".to_string(), Some(42));
    input.insert("none".to_string(), None);
    let result = option_roundtrip(&input);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get("some"), Some(&Some(42)));
    assert_eq!(result.get("none"), Some(&None));
}

fn test_record_roundtrip() {
    let mut values = NamesById::new();
    values.insert(10, "ten".to_string());
    values.insert(20, "twenty".to_string());
    let entry = LabeledEntry {
        label: "test-label".to_string(),
        values,
    };
    let result = record_roundtrip(&entry);
    assert_eq!(result.label, "test-label");
    assert_eq!(result.values.len(), 2);
    assert_eq!(result.values.get(&10).map(String::as_str), Some("ten"));
    assert_eq!(result.values.get(&20).map(String::as_str), Some("twenty"));
}

fn test_inline_roundtrip() {
    let mut input = wit_bindgen::rt::Map::new();
    input.insert(1, "one".to_string());
    input.insert(2, "two".to_string());
    let result = inline_roundtrip(&input);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get("one"), Some(&1));
    assert_eq!(result.get("two"), Some(&2));
}

fn test_large_map() {
    let mut input = NamesById::new();
    for i in 0..100 {
        input.insert(i, format!("value-{i}"));
    }
    let result = large_roundtrip(&input);
    assert_eq!(result.len(), 100);
    for i in 0..100 {
        assert_eq!(result.get(&i).map(String::as_str), Some(format!("value-{i}").as_str()));
    }
}
