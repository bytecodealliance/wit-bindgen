include!(env!("BINDINGS"));

use crate::exports::a::b::the_test::Guest;
use std::cell::Cell;
use wit_bindgen::rt::async_support::FutureReader;

struct Component;

export!(Component);

std::thread_local!(
    static SLOT: Cell<Option<FutureReader<()>>> = const { Cell::new(None) };
);

impl Guest for Component {
    fn set(future: FutureReader<()>) {
        SLOT.with(|s| s.set(Some(future)));
    }
    fn get() -> FutureReader<()> {
        SLOT.with(|s| s.replace(None).unwrap())
    }
}
