include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        crate::test::common::i_runner::f().await;
    }
}
