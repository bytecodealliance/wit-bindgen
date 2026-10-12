use futures::channel::oneshot;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::task::Poll;
use wit_bindgen::{FutureReader, spawn_local};

include!(env!("BINDINGS"));

use crate::exports::test::rust_cancel_wake::i::{Guest, Status};

struct Component;

export!(Component);

static POLLS: AtomicU32 = AtomicU32::new(0);
static DROPPED: AtomicBool = AtomicBool::new(false);

struct SendOnDrop(Option<oneshot::Sender<()>>);

impl Drop for SendOnDrop {
    fn drop(&mut self) {
        // Sending wakes the spawned future's registered oneshot receiver.
        self.0.take().unwrap().send(()).unwrap();
        DROPPED.store(true, Relaxed);
    }
}

impl Guest for Component {
    async fn cancellable(checkpoint: FutureReader<()>) {
        let (sender, mut receiver) = oneshot::channel();
        let _sender = SendOnDrop(Some(sender));
        spawn_local(async move {
            poll_fn(|cx| {
                POLLS.fetch_add(1, Relaxed);
                Pin::new(&mut receiver).poll(cx)
            })
            .await
            .unwrap();
            unreachable!("the cancelled waiter must not resume");
        });

        // FuturesUnordered destroys the most recently polled future first.
        // Poll the root last so its sender wakes the still-live waiter.
        checkpoint.await;
        poll_fn(|_| {
            POLLS.fetch_add(1, Relaxed);
            Poll::<()>::Pending
        })
        .await;
        unreachable!("the cancelled task must not resume");
    }

    fn get_status() -> Status {
        Status {
            polls: POLLS.load(Relaxed),
            dropped: DROPPED.load(Relaxed),
        }
    }
}
