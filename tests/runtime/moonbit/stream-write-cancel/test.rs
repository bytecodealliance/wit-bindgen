include!(env!("BINDINGS"));

use crate::exports::test::moonbit_stream_write_cancel::controller::Guest;
use crate::test::moonbit_stream_write_cancel::holder;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Poll, Waker};

struct Component;

export!(Component);

static CANCELLATION_OBSERVED: AtomicBool = AtomicBool::new(false);
static CANCELLATION_WAKER: Mutex<Option<Waker>> = Mutex::new(None);
static SECOND_WRITE_STARTED: AtomicBool = AtomicBool::new(false);
static PRODUCER_FINISHED: AtomicBool = AtomicBool::new(false);
static PRODUCER_WAKER: Mutex<Option<Waker>> = Mutex::new(None);

struct CancellationGuard;

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        CANCELLATION_OBSERVED.store(true, Ordering::SeqCst);
        if let Some(waker) = CANCELLATION_WAKER.lock().unwrap().take() {
            waker.wake();
        }
    }
}

struct ProducerGuard;

impl Drop for ProducerGuard {
    fn drop(&mut self) {
        PRODUCER_FINISHED.store(true, Ordering::SeqCst);
        if let Some(waker) = PRODUCER_WAKER.lock().unwrap().take() {
            waker.wake();
        }
    }
}

impl Guest for Component {
    async fn run_until_cancelled() {
        let _guard = CancellationGuard;
        let (mut writer, reader) = wit_stream::new::<holder::Leaf>();
        holder::hold(reader).await;
        holder::mark_write_started();
        let _ = writer
            .write(vec![holder::Leaf::new(), holder::Leaf::new()])
            .await;
        std::future::pending::<()>().await;
    }

    fn cancellation_observed() -> bool {
        CANCELLATION_OBSERVED.load(Ordering::SeqCst)
    }

    async fn wait_cancellation_observed() {
        std::future::poll_fn(|cx| {
            if CANCELLATION_OBSERVED.load(Ordering::SeqCst) {
                return Poll::Ready(());
            }

            *CANCELLATION_WAKER.lock().unwrap() = Some(cx.waker().clone());
            if CANCELLATION_OBSERVED.load(Ordering::SeqCst) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await
    }

    async fn start_peer_drop() {
        // The MoonBit implementation primes its language-level queue locally.
        // A CM stream cannot be read and written by the same component when
        // its payload contains resources, so account for that local item here.
        drop(holder::Leaf::new());

        let (mut writer, reader) = wit_stream::new::<holder::Leaf>();
        wit_bindgen::spawn_local(async move {
            let _guard = ProducerGuard;

            assert!(writer.write_one(holder::Leaf::new()).await.is_none());
            SECOND_WRITE_STARTED.store(true, Ordering::SeqCst);
            assert!(writer.write_one(holder::Leaf::new()).await.is_some());
            assert!(writer.write_one(holder::Leaf::new()).await.is_some());
        });

        holder::hold(reader).await;
    }

    fn second_write_started() -> bool {
        SECOND_WRITE_STARTED.load(Ordering::SeqCst)
    }

    fn producer_finished() -> bool {
        PRODUCER_FINISHED.load(Ordering::SeqCst)
    }

    async fn wait_producer_finished() {
        std::future::poll_fn(|cx| {
            if PRODUCER_FINISHED.load(Ordering::SeqCst) {
                return Poll::Ready(());
            }

            *PRODUCER_WAKER.lock().unwrap() = Some(cx.waker().clone());
            if PRODUCER_FINISHED.load(Ordering::SeqCst) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await
    }
}
