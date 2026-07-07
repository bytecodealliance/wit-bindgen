//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use wit_bindgen::block_on;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (writer, reader) = wit_stream::new::<u8>();
        let reader = a::b::i::launder(reader);
        let noop_cx = &mut Context::from_waker(Waker::noop());

        let mut w1 = pin!(async {
            let mut w = writer;
            let _ = w.write(vec![1u8]).await;
            w
        });

        // Step 1 — register &w1.completion_status in export_task.waitables[H].
        assert!(matches!(w1.as_mut().poll(noop_cx), Poll::Pending));

        // Step 2 — block_on completes w1; export_task.waitables[H] goes stale.
        // _reader must stay alive so the step-3 write blocks (Dropped skips
        // register_waker and hides the bug).
        let (writer, _reader) = block_on(async move {
            let mut reader = reader;
            let (w, _) = futures::join!(w1, reader.read(Vec::with_capacity(1)));
            (w, reader)
        });

        // Step 3 — register &w2.completion_status; gets freed
        // &w1.completion_status back as prev → assert_eq!(ptr, prev.cast())
        // panics at waitable.rs:201.
        let mut w2 = pin!(async move {
            let mut w = writer;
            let pad = [0u64; 16];
            let _ = w.write(vec![2u8]).await;
            let _ = pad; // explicit use after await keeps pad in the state machine
        });
        let _ = w2.as_mut().poll(noop_cx); // panics
    }
}
