wai_bindgen_rust::import!("crates/nested_a/nested_a.wai");
wai_bindgen_rust::export!("crates/nested_b/nested_b.wai");

struct NestedB;

impl nested_b::NestedB for NestedB {
    fn outer(x: String) -> String {
        nested_a::inner(&x)
    }
}
