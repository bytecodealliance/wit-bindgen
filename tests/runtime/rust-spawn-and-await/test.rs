include!(env!("BINDINGS"));

use futures::channel::oneshot;
use std::cell::RefCell;
use wit_bindgen::{Task, spawn_local};

struct Component;

export!(Component);

std::thread_local! {
    static TASK: RefCell<Option<Task<u32>>> = const { RefCell::new(None) };
    // Send through this channel to resolve the `Task`.
    static RESOLVE_CHANNEL: RefCell<Option<oneshot::Sender<()>>> = const { RefCell::new(None) };
    // Side channel to check that the `Task` has resolved without explicitly awaiting it.
    static ACK_CHANNEL: RefCell<Option<oneshot::Receiver<()>>> = const { RefCell::new(None) };
}

impl crate::exports::test::rust_spawn_and_await::i::Guest for Component {
    async fn start() {
        let (tx, rx) = oneshot::channel();
        let (ack_tx, ack_rx) = oneshot::channel();
        let task = spawn_local(async {
            rx.await.unwrap();
            let _ = ack_tx.send(());
            42
        });
        TASK.with(|slot| assert!(slot.replace(Some(task)).is_none()));
        RESOLVE_CHANNEL.with(|slot| assert!(slot.replace(Some(tx)).is_none()));
        ACK_CHANNEL.with(|slot| slot.replace(Some(ack_rx)));
        std::future::pending::<()>().await;
    }

    async fn await_task() -> Option<u32> {
        let task = TASK.with(|slot| slot.borrow_mut().take().unwrap());
        task.await
    }

    async fn cancel_task() -> Option<u32> {
        let task = TASK.with(|slot| slot.borrow_mut().take().unwrap());
        task.cancel().await
    }

    async fn resolve() {
        let channel = RESOLVE_CHANNEL.with(|slot| slot.borrow_mut().take().unwrap());
        // Ignore error when trying to resolve the `Task` because some tests
        // cancel it before it completes.
        let _ = channel.send(());
    }

    async fn await_resolve() {
        let channel = ACK_CHANNEL.with(|slot| slot.borrow_mut().take().unwrap());
        // Ignore error when trying to resolve the `Task` because some tests
        // cancel it before it completes.
        channel.await.unwrap();
    }
}
