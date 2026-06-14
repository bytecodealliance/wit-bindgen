//@ [lang]
//@ link_shared = true

include!(env!("BINDINGS"));

use crate::exports::my::inline::a;
use wit_bindgen::FutureReader;

struct Component;
export!(Component);

impl a::Guest for Component {
    async fn a(f: FutureReader<u32>) -> u32 {
        f.await
    }
}
