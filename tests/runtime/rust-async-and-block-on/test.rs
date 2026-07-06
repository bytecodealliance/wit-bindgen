include!(env!("BINDINGS"));

use wit_bindgen::StreamReader;

struct Component;

export!(Component);

impl crate::exports::a::b::i::Guest for Component {
    fn launder(x: StreamReader<u8>) -> StreamReader<u8> {
        x
    }
}
