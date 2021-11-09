wai_bindgen_rust::export!("crates/nested_a/nested_a.wai");

struct NestedA;

impl nested_a::NestedA for NestedA {
    fn inner(x: String) -> String {
        x
    }
}
