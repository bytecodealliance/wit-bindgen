include!(env!("BINDINGS"));

use std::cell::Cell;
use std::rc::Rc;
use wit_bindgen::{block_on, spawn_local};

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let ran = Rc::new(Cell::new(0));
        let count = || {
            let ran = ran.clone();
            async move { ran.set(ran.get() + 1) }
        };

        let before = spawn_local(count());
        let nested = block_on(async {
            foo::hi().await;
            spawn_local(async { 7 }).await
        });
        assert_eq!(nested, Some(7));
        assert_eq!(ran.get(), 0, "`block_on` ran a task spawned outside of it");

        before.await;
        assert_eq!(ran.get(), 1);
    }
}
