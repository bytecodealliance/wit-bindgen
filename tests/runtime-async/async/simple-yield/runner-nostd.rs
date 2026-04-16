//@ wasmtime-flags = '-Wcomponent-model-async'
//@ args = ['--std-feature']

#![no_std]

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        crate::a::b::i::f().await;
    }
}
