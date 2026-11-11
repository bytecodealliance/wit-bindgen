include!(env!("BINDINGS"));

use crate::exports::a::b::the_test::Guest;
use std::sync::Mutex;
use wit_bindgen::rt::async_support::FutureReader;

struct Component;

export!(Component);

static SLOT: Mutex<Option<FutureReader<()>>> = Mutex::new(None);

impl Guest for Component {
    fn set(future: FutureReader<()>) {
        *SLOT.lock().unwrap() = Some(future);
    }
    fn get() -> FutureReader<()> {
        SLOT.lock().unwrap().take().unwrap()
    }
}
