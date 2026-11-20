use std::sync::Mutex;
use wit_bindgen::{FutureReader, StreamReader, StreamWriter};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

static WRITER: Mutex<Option<StreamWriter<()>>> = Mutex::new(None);

impl crate::exports::my::test::i::Guest for Component {
    async fn pending_future(x: FutureReader<()>) {
        x.await
    }

    async fn pending_stream(mut x: StreamReader<()>) {
        x.next().await;
    }

    fn make_stream() -> StreamReader<()> {
        let (tx, rx) = wit_stream::new();
        *WRITER.lock().unwrap() = Some(tx);
        rx
    }

    async fn pending_stream_write() {
        let mut tx = WRITER.lock().unwrap().take().unwrap();
        tx.write_one(()).await;
    }
}
