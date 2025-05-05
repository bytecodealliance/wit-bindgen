include!(env!("BINDINGS"));

use crate::my::test::i::*;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;
use wit_bindgen::yield_async;

fn main() {
    println!("test cancelling an import in progress");
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new();
        let mut import = Box::pin(pending_import(rx));
        assert!(import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        drop(import);
        tx.write(()).await.unwrap_err();
    });

    println!("test cancelling an import before it starts");
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new();
        let import = Box::pin(pending_import(rx));
        drop(import);
        tx.write(()).await.unwrap_err();
    });

    println!("test cancelling an import in the started state");
    wit_bindgen::block_on(async {
        let (tx1, rx1) = wit_future::new();
        let (tx2, rx2) = wit_future::new();

        // create a task in the "started" state, but don't complete it yet
        let mut started_import = Box::pin(pending_import(rx1));
        assert!(started_import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());

        // request the other component sets its backpressure flag meaning we
        // won't be able to create new tasks in the "started" state.
        backpressure_set(true);
        let mut starting_import = Box::pin(pending_import(rx2));
        assert!(starting_import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());

        // Now cancel the "starting" import. This should notably drop handles in
        // arguments since they get re-acquired during cancellation
        drop(starting_import);

        // cancel our in-progress export
        drop(started_import);

        backpressure_set(false);

        // both channels should be closed
        tx1.write(()).await.unwrap_err();
        tx2.write(()).await.unwrap_err();
    });

    // Race an import's cancellation with a status code saying it's done.
    println!("test cancellation with a status code saying it's done");
    wit_bindgen::block_on(async {
        // Start a subtask and get it into the "started" state
        let (tx, rx) = wit_future::new();
        let mut import = Box::pin(pending_import(rx));
        assert!(import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());

        // Complete the subtask, but don't see the completion in Rust yet.
        tx.write(()).await.unwrap();

        // Let the subtask's completion notification make its way to our task
        // here.
        for _ in 0..5 {
            yield_async().await;
        }

        // Now cancel the import, despite it actually being done. This should
        // realize that the cancellation is racing completion.
        drop(import);
    });

    // Race an import's cancellation with a pending status code indicating that
    // it's transitioning from started => returned.
    println!("race cancellation with pending status code");
    wit_bindgen::block_on(async {
        // Start a subtask and get it into the "started" state
        let (tx1, rx1) = wit_future::new();
        let mut started_import = Box::pin(pending_import(rx1));
        assert!(started_import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());

        // force the next subtask to start out in the "starting" state, not the
        // "started" state.
        backpressure_set(true);
        let (tx2, rx2) = wit_future::new();
        let mut starting_import = Box::pin(pending_import(rx2));
        assert!(starting_import
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());

        // Disable backpressure in the other component which will let the
        // `starting_import`, previously in the "STARTING" state, get a queued up
        // notification that it's entered the "STARTED" state.
        backpressure_set(false);
        for _ in 0..5 {
            yield_async().await;
        }

        // Now cancel the `starting_import`. This should correctly pick up the
        // STARTING => STARTED state transition and handle that correctly.
        drop(starting_import);

        // Our future to the import we cancelled shouldn't be able to complete
        // its write.
        tx2.write(()).await.unwrap_err();

        // Complete the other import normally just to assert that it's not
        // cancelled and able to proceed as usual.
        tx1.write(()).await.unwrap();
        started_import.await;
    });
}
