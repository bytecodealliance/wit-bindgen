include!(env!("BINDINGS"));

use crate::my::test::i::{read_and_drop, take_then_drop};
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;
use wit_bindgen::FutureWriteCancel;

fn main() {
    wit_bindgen::block_on(async {
        // cancel from the other end
        let (tx, rx) = wit_future::new(|| unreachable!());
        let f1 = async { tx.write("hello".into()).await };
        let f2 = async { take_then_drop(rx) };
        let (result, ()) = futures::join!(f1, f2);
        assert_eq!(result.unwrap_err().value, "hello");

        // cancel before we actually hit the intrinsic
        let (tx, _rx) = wit_future::new::<String>(|| String::new());
        let mut future = Box::pin(tx.write("hello2".into()));
        let tx = match future.as_mut().cancel() {
            FutureWriteCancel::Cancelled(val, tx) => {
                assert_eq!(val, "hello2");
                tx
            }
            _ => unreachable!(),
        };

        // cancel after we hit the intrinsic
        let mut future = Box::pin(tx.write("hello3".into()));
        assert!(future
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        match future.as_mut().cancel() {
            FutureWriteCancel::Cancelled(val, _) => {
                assert_eq!(val, "hello3");
            }
            _ => unreachable!(),
        };

        // cancel after we hit the intrinsic and then drop the other end
        let (tx, rx) = wit_future::new::<String>(|| unreachable!());
        let mut future = Box::pin(tx.write("hello3".into()));
        assert!(future
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        drop(rx);
        match future.as_mut().cancel() {
            FutureWriteCancel::Dropped(val) => assert_eq!(val, "hello3"),
            other => panic!("expected dropped, got: {other:?}"),
        };

        // Start a write, wait for it to be pending, then go complete the write
        // in some async work, then cancel it and witness that it was written,
        // not cancelled.
        let (tx, rx) = wit_future::new::<String>(|| unreachable!());
        let mut future = Box::pin(tx.write("hello3".into()));
        assert!(future
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        read_and_drop(rx).await;
        match future.as_mut().cancel() {
            FutureWriteCancel::AlreadySent => {}
            other => panic!("expected sent, got: {other:?}"),
        };
    });
}
