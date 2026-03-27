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
}
