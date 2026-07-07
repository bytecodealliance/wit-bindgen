include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::a::b::i::Guest for Component {
    async fn f() {}
}
