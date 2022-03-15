wit_bindgen_rust::export!("nested_a.wit");

struct NestedA;

impl nested_a::NestedA for NestedA {
    fn inner(x: String) -> String {
        x
    }
}
