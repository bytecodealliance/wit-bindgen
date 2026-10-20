include!(env!("BINDINGS"));

use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

static CTOR_RUNS: AtomicU32 = AtomicU32::new(0);

extern "C" fn ctor() {
    CTOR_RUNS.fetch_add(1, Relaxed);
}

#[used]
#[unsafe(link_section = ".init_array")]
static CTOR: extern "C" fn() = ctor;

struct Test;

export!(Test);

impl exports::the::test::i::Guest for Test {
    fn ctor_runs() -> u32 {
        CTOR_RUNS.load(Relaxed)
    }
}
