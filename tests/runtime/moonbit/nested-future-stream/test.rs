include!(env!("BINDINGS"));

use crate::exports::test::moonbit_nested_future_stream::nested::Guest;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Poll, Waker};
use wit_bindgen::{FutureReader, StreamReader, StreamResult};

struct Component;

export!(Component);

static SHARED: Mutex<Option<u32>> = Mutex::new(None);
static SHARED_WAKER: Mutex<Option<Waker>> = Mutex::new(None);
static CANCELLATION_OBSERVED: AtomicBool = AtomicBool::new(false);
static CANCELLATION_WAKER: Mutex<Option<Waker>> = Mutex::new(None);

struct CancellationGuard;

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        CANCELLATION_OBSERVED.store(true, Ordering::SeqCst);
        if let Some(waker) = CANCELLATION_WAKER.lock().unwrap().take() {
            waker.wake();
        }
    }
}

impl Guest for Component {
    async fn relay(
        value: FutureReader<FutureReader<StreamReader<u8>>>,
    ) -> FutureReader<FutureReader<StreamReader<u8>>> {
        let (outer_writer, outer_reader) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn_local(async move {
            let input_inner = value.await;
            let (inner_writer, inner_reader) = wit_future::new(|| unreachable!());
            let outer_open = outer_writer.write(inner_reader).await.is_ok();

            // Continue consuming the input chain after an outer rejection so
            // that its producer observes the rejection at the innermost stream.
            let mut input_stream = input_inner.await;
            let (mut output_writer, output_reader) = wit_stream::new();
            let inner_open = inner_writer.write(output_reader).await.is_ok();
            if !outer_open || !inner_open {
                return;
            }

            loop {
                let (result, values) = input_stream.read(Vec::with_capacity(16)).await;
                if !values.is_empty() && !output_writer.write_all(values).await.is_empty() {
                    return;
                }
                match result {
                    StreamResult::Complete(_) => {}
                    StreamResult::Dropped => return,
                    StreamResult::Cancelled => unreachable!(),
                }
            }
        });
        outer_reader
    }

    async fn relay_stream(value: StreamReader<FutureReader<u8>>) -> StreamReader<FutureReader<u8>> {
        let (mut output_writer, output_reader) = wit_stream::new();
        wit_bindgen::spawn_local(async move {
            let mut input = value;
            loop {
                let (result, values) = input.read(Vec::with_capacity(1)).await;
                for input_value in values {
                    let (value_writer, value_reader) = wit_future::new(|| unreachable!());
                    if output_writer.write_one(value_reader).await.is_some() {
                        let _ = input_value.await;
                        return;
                    }
                    let value = input_value.await;
                    let _ = value_writer.write(value).await;
                }
                match result {
                    StreamResult::Complete(_) => {}
                    StreamResult::Dropped => return,
                    StreamResult::Cancelled => unreachable!(),
                }
            }
        });
        output_reader
    }

    async fn concurrent_writes() -> StreamReader<u8> {
        let (mut writer, reader) = wit_stream::new();
        wit_bindgen::spawn_local(async move {
            assert!(writer.write_all(vec![1, 2]).await.is_empty());
        });
        reader
    }

    async fn post_return_lazy() -> StreamReader<u8> {
        let (mut writer, reader) = wit_stream::new();
        wit_bindgen::spawn_local(async move {
            assert!(writer.write_one(42).await.is_none());
        });
        reader
    }

    async fn wait_shared() -> u32 {
        std::future::poll_fn(|cx| {
            if let Some(value) = SHARED.lock().unwrap().take() {
                return Poll::Ready(value);
            }

            *SHARED_WAKER.lock().unwrap() = Some(cx.waker().clone());
            match SHARED.lock().unwrap().take() {
                Some(value) => Poll::Ready(value),
                None => Poll::Pending,
            }
        })
        .await
    }

    async fn resolve_shared(value: u32) {
        assert!(SHARED.lock().unwrap().replace(value).is_none());
        if let Some(waker) = SHARED_WAKER.lock().unwrap().take() {
            waker.wake();
        }
    }

    async fn wait_cancelled() {
        let _guard = CancellationGuard;
        std::future::pending::<()>().await;
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

    fn cancellation_observed() -> bool {
        CANCELLATION_OBSERVED.load(Ordering::SeqCst)
    }
}
