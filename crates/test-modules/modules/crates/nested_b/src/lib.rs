wit_bindgen_rust::import!("crates/nested_a/nested_a.wit");
wit_bindgen_rust::export!("crates/nested_b/nested_b.wit");

struct NestedB;

impl nested_b::NestedB for NestedB {
    fn outer(x: String) -> String {
        nested_a::inner(&x)
    }
}
