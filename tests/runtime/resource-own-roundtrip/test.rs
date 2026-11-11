include!(env!("BINDINGS"));

use exports::test::resource_own_roundtrip::to_test::{Guest, GuestThing, Thing};
use std::sync::atomic::{AtomicU32, Ordering};

static NUM_DROPPED: AtomicU32 = AtomicU32::new(0);

pub struct Test {}

export!(Test);

pub struct MyThing {
    val: u32,
}

impl Drop for MyThing {
    fn drop(&mut self) {
        NUM_DROPPED.fetch_add(1, Ordering::SeqCst);
    }
}

impl Guest for Test {
    type Thing = MyThing;

    fn consume_both(a: Thing, b: Thing) -> u32 {
        a.get::<MyThing>().val + b.get::<MyThing>().val
    }

    fn num_dropped() -> u32 {
        NUM_DROPPED.load(Ordering::SeqCst)
    }
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self { val: v }
    }
}
