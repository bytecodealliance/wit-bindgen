use futures::channel::oneshot;
use std::sync::Mutex;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

static TX: Mutex<Option<oneshot::Sender<()>>> = Mutex::new(None);

impl crate::exports::my::test::i::Guest for Component {
    async fn pending_import() {
        let (tx, rx) = oneshot::channel::<()>();
        *TX.lock().unwrap() = Some(tx);
        rx.await.unwrap();
    }

    fn resolve_pending_import() {
        TX.lock().unwrap().take().unwrap().send(()).unwrap();
    }
}
