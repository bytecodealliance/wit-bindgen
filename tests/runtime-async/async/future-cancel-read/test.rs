use wit_bindgen::FutureReader;

use futures::task::noop_waker_ref;
use std::future::{Future, IntoFuture};
use std::task::Context;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn cancel_before_read(x: FutureReader<u32>) {
        let mut read = Box::pin(x.into_future());
        let reader = read.as_mut().cancel().unwrap_err();
        drop(reader);
    }

    async fn cancel_after_read(x: FutureReader<u32>) {
        let mut read = Box::pin(x.into_future());
        assert!(read
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        let reader = read.as_mut().cancel().unwrap_err();
        drop(reader);
    }

    async fn start_read_then_cancel() {
        let (tx, rx) = wit_future::new::<u32>();
        let mut read = Box::pin(rx.into_future());
        assert!(read
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        drop(tx);
        assert!(read.as_mut().cancel().unwrap().is_none());
    }
}
