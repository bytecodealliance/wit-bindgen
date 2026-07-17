//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::test::moonbit_stream_write_cancel::controller::{
    cancellation_observed, producer_finished, run_until_cancelled, second_write_started,
    start_peer_drop, wait_cancellation_observed, wait_producer_finished,
};
use crate::test::moonbit_stream_write_cancel::holder;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let mut task = Box::pin(run_until_cancelled());
        assert!(task
            .as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending());
        holder::wait_write_started().await;
        assert!(holder::write_started());

        drop(task);
        wait_cancellation_observed().await;
        assert!(cancellation_observed());
        assert!(holder::writable_dropped().await);
        assert_eq!(holder::leaf_live_count(), 0);
        assert_eq!(holder::leaf_drop_count(), 1);

        start_peer_drop().await;
        assert!(holder::read_one_and_drop().await);
        wait_producer_finished().await;
        assert!(second_write_started());
        assert!(producer_finished());
        assert_eq!(holder::leaf_live_count(), 0);
        assert_eq!(holder::leaf_drop_count(), 5);
    }
}
