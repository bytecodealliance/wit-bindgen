include!(env!("BINDINGS"));

fn main() {
    wit_bindgen::block_on(async {
        crate::a::b::i::f().await;
    });
}
