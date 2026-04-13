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
        test_multi_param_roundtrip();
        test_nested_roundtrip();
        test_variant_roundtrip();
        test_result_roundtrip();
        test_tuple_roundtrip();
        test_single_entry_roundtrip();
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
        assert_eq!(
            result.get(&i).map(String::as_str),
            Some(format!("value-{i}").as_str()),
        );
    }
}

fn test_multi_param_roundtrip() {
    let mut names = NamesById::new();
    names.insert(1, "one".to_string());
    names.insert(2, "two".to_string());
    let mut bytes = BytesByName::new();
    bytes.insert("key".to_string(), vec![42u8]);
    let (ids, bytes_out) = multi_param_roundtrip(&names, &bytes);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids.get("one"), Some(&1));
    assert_eq!(ids.get("two"), Some(&2));
    assert_eq!(bytes_out.len(), 1);
    assert_eq!(
        bytes_out.get("key").map(Vec::as_slice),
        Some([42u8].as_slice()),
    );
}

fn test_nested_roundtrip() {
    let mut inner_a = wit_bindgen::rt::Map::new();
    inner_a.insert(1, "one".to_string());
    inner_a.insert(2, "two".to_string());
    let mut inner_b = wit_bindgen::rt::Map::new();
    inner_b.insert(10, "ten".to_string());
    let mut outer = wit_bindgen::rt::Map::new();
    outer.insert("group-a".to_string(), inner_a);
    outer.insert("group-b".to_string(), inner_b);
    let result = nested_roundtrip(&outer);
    assert_eq!(result.len(), 2);
    let ra = result.get("group-a").unwrap();
    assert_eq!(ra.get(&1).map(String::as_str), Some("one"));
    assert_eq!(ra.get(&2).map(String::as_str), Some("two"));
    let rb = result.get("group-b").unwrap();
    assert_eq!(rb.get(&10).map(String::as_str), Some("ten"));
}

fn test_variant_roundtrip() {
    let mut map = NamesById::new();
    map.insert(1, "one".to_string());
    let as_map = variant_roundtrip(&MapOrString::AsMap(map));
    match &as_map {
        MapOrString::AsMap(m) => {
            assert_eq!(m.get(&1).map(String::as_str), Some("one"));
        }
        MapOrString::AsString(_) => panic!("expected AsMap"),
    }

    let as_str = variant_roundtrip(&MapOrString::AsString("hello".to_string()));
    match &as_str {
        MapOrString::AsString(s) => assert_eq!(s, "hello"),
        MapOrString::AsMap(_) => panic!("expected AsString"),
    }
}

fn test_result_roundtrip() {
    let mut map = NamesById::new();
    map.insert(5, "five".to_string());
    let ok_result = result_roundtrip(Ok(&map));
    match &ok_result {
        Ok(m) => assert_eq!(m.get(&5).map(String::as_str), Some("five")),
        Err(_) => panic!("expected Ok"),
    }

    let err_result = result_roundtrip(Err("bad input"));
    match &err_result {
        Err(e) => assert_eq!(e, "bad input"),
        Ok(_) => panic!("expected Err"),
    }
}

fn test_tuple_roundtrip() {
    let mut map = NamesById::new();
    map.insert(7, "seven".to_string());
    let (result_map, result_num) = tuple_roundtrip((&map, 42));
    assert_eq!(result_map.len(), 1);
    assert_eq!(result_map.get(&7).map(String::as_str), Some("seven"));
    assert_eq!(result_num, 42);
}

fn test_single_entry_roundtrip() {
    let mut input = NamesById::new();
    input.insert(99, "ninety-nine".to_string());
    let result = single_entry_roundtrip(&input);
    assert_eq!(result.len(), 1);
    assert_eq!(
        result.get(&99).map(String::as_str),
        Some("ninety-nine"),
    );
}
