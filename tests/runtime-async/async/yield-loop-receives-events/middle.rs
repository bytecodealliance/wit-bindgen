include!(env!("BINDINGS"));

use crate::test::common::i_middle::f;
use std::task::Poll;

pub struct Component;

export!(Component);

static mut HIT: bool = false;

impl crate::exports::test::common::i_runner::Guest for Component {
    async fn f() {
        wit_bindgen::spawn(async move {
            f().await;
            unsafe {
                HIT = true;
            }
        });

        // This is an "infinite loop" but it's also effectively a yield which
        // should enable not only making progress on sibling rust-level tasks
        // but additionally async events should be deliverable.
        std::future::poll_fn(|cx| unsafe {
            if HIT {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }
}
