include!(env!("BINDINGS"));

use crate::exports::test::list_in_variant::to_test::*;

struct Component;

export!(Component);

impl exports::test::list_in_variant::to_test::Guest for Component {
    fn list_in_option(data: Option<Vec<String>>) -> String {
        match data {
            Some(list) => list.join(","),
            None => "none".to_string(),
        }
    }

    fn list_in_variant(data: PayloadOrEmpty) -> String {
        match data {
            PayloadOrEmpty::WithData(list) => list.join(","),
            PayloadOrEmpty::Empty => "empty".to_string(),
        }
    }

    fn list_in_result(data: Result<Vec<String>, String>) -> String {
        match data {
            Ok(list) => list.join(","),
            Err(e) => format!("err:{}", e),
        }
    }

    fn list_in_option_with_return(data: Option<Vec<String>>) -> Summary {
        match data {
            Some(list) => Summary {
                count: list.len() as u32,
                label: list.join(","),
            },
            None => Summary {
                count: 0,
                label: "none".to_string(),
            },
        }
    }

    fn top_level_list(items: Vec<String>) -> String {
        items.join(",")
    }

    fn canonical_list_params(
        bytes: Vec<u8>,
        unsigned_shorts: Vec<u16>,
        signed_shorts: Vec<i16>,
        words: Vec<u32>,
        nested: CanonicalLists,
    ) -> u32 {
        assert_eq!(bytes, [1, 2, 3, 4]);
        assert_eq!(unsigned_shorts, [100, 200]);
        assert_eq!(signed_shorts, [-10, 20]);
        assert_eq!(words, [10, 20, 30]);
        assert_eq!(nested.bytes, [5, 6, 7]);
        assert_eq!(nested.unsigned_shorts, [300, 400]);
        assert_eq!(nested.signed_shorts, [-30, 40]);
        assert_eq!(nested.words, [40, 50]);
        172
    }
}
