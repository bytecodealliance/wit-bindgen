include!(env!("BINDINGS"));

fn main() {
    wit_bindgen::block_on(async {
        crate::test::common::i_runner::f().await;
    });
}
