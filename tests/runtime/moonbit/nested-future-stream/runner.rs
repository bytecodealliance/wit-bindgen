//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::test::moonbit_nested_future_stream::nested::{
    cancellation_observed, concurrent_writes, post_return_lazy, relay, relay_stream,
    resolve_shared, wait_cancellation_observed, wait_cancelled, wait_shared,
};
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;
use wit_bindgen::{FutureReader, StreamReader, StreamResult};

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        transfers_nested_endpoints().await;
        rejects_nested_endpoints().await;
        transfers_stream_of_futures().await;
        serializes_concurrent_stream_writes().await;
        wakes_a_different_component_task().await;
        materializes_a_lazy_stream_after_return().await;
        cancels_a_moonbit_component_task().await;
    }
}

async fn cancels_a_moonbit_component_task() {
    let mut task = Box::pin(wait_cancelled());
    assert!(task
        .as_mut()
        .poll(&mut Context::from_waker(noop_waker_ref()))
        .is_pending());
    drop(task);
    wait_cancellation_observed().await;
    assert!(cancellation_observed());
}

async fn wakes_a_different_component_task() {
    let (value, ()) = futures::join!(wait_shared(), resolve_shared(42));
    assert_eq!(value, 42);
}

async fn materializes_a_lazy_stream_after_return() {
    let mut stream = post_return_lazy().await;
    let (result, values) = stream.read(Vec::with_capacity(1)).await;
    assert_eq!(result, StreamResult::Complete(1));
    assert_eq!(values, [42]);
}

async fn serializes_concurrent_stream_writes() {
    let mut stream = concurrent_writes().await;
    for _ in 0..4 {
        wit_bindgen::yield_async().await;
    }
    let (first_result, first) = stream.read(Vec::with_capacity(1)).await;
    assert_eq!(first_result, StreamResult::Complete(1));
    let (second_result, second) = stream.read(Vec::with_capacity(1)).await;
    assert_eq!(second_result, StreamResult::Complete(1));
    let mut values = vec![first[0], second[0]];
    values.sort();
    assert_eq!(values, [1, 2]);
}

async fn transfers_stream_of_futures() {
    let (mut input_writer, input) = wit_stream::new::<FutureReader<u8>>();
    let (value_writer1, value_reader1) = wit_future::new::<u8>(|| unreachable!());
    let (value_writer2, value_reader2) = wit_future::new::<u8>(|| unreachable!());
    let mut output = relay_stream(input).await;

    let send = async {
        let (result, remaining) = input_writer.write(vec![value_reader1]).await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(remaining.remaining(), 0);
        value_writer1.write(11).await.unwrap();

        let (result, remaining) = input_writer.write(vec![value_reader2]).await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(remaining.remaining(), 0);
        value_writer2.write(22).await.unwrap();
        drop(input_writer);
    };
    let receive = async {
        let (result, values) = output.read(Vec::with_capacity(1)).await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(values.into_iter().next().unwrap().await, 11);

        let (result, values) = output.read(Vec::with_capacity(1)).await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(values.into_iter().next().unwrap().await, 22);
    };
    let ((), ()) = futures::join!(send, receive);
}

async fn transfers_nested_endpoints() {
    let (mut stream_writer, stream_reader) = wit_stream::new::<u8>();
    let (inner_writer, inner_reader) =
        wit_future::new::<StreamReader<u8>>(|| unreachable!());
    let (outer_writer, outer_reader) =
        wit_future::new::<FutureReader<StreamReader<u8>>>(|| unreachable!());
    let output = relay(outer_reader).await;

    let send = async {
        outer_writer.write(inner_reader).await.unwrap();
        inner_writer.write(stream_reader).await.unwrap();
        let (result, remaining) = stream_writer.write(vec![1, 2, 3]).await;
        assert_eq!(result, StreamResult::Complete(3));
        assert_eq!(remaining.remaining(), 0);
        drop(stream_writer);
    };
    let receive = async {
        let mut stream = output.await.await;
        let (result, bytes) = stream.read(Vec::with_capacity(3)).await;
        assert_eq!(result, StreamResult::Complete(3));
        assert_eq!(bytes, [1, 2, 3]);
    };
    let ((), ()) = futures::join!(send, receive);
}

async fn rejects_nested_endpoints() {
    let (mut stream_writer, stream_reader) = wit_stream::new::<u8>();
    let (inner_writer, inner_reader) =
        wit_future::new::<StreamReader<u8>>(|| unreachable!());
    let (outer_writer, outer_reader) =
        wit_future::new::<FutureReader<StreamReader<u8>>>(|| unreachable!());
    let output = relay(outer_reader).await;
    drop(output);

    outer_writer.write(inner_reader).await.unwrap();
    inner_writer.write(stream_reader).await.unwrap();
    let (result, remaining) = stream_writer.write(vec![9]).await;
    match result {
        StreamResult::Dropped => assert_eq!(remaining.remaining(), 1),
        StreamResult::Complete(1) => {
            assert_eq!(remaining.remaining(), 0);
            let (result, remaining) = stream_writer.write(vec![10]).await;
            assert_eq!(result, StreamResult::Dropped);
            assert_eq!(remaining.remaining(), 1);
        }
        result => panic!("unexpected stream write result: {result:?}"),
    }
}
