//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;

struct Component;

export!(Component);

const N: usize = 2000;

macro_rules! check_no_leak {
    ($name:expr, $body:block) => {{
        println!("{}", $name);
        for _ in 0..N $body
        let before = heap_mark();
        for _ in 0..N $body
        assert_eq!(heap_mark(), before, "{}", $name);
    }};
}

impl Guest for Component {
    async fn run() {
        check_no_leak!("finished calls free their arguments", {
            many_arguments(
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
                "g".into(),
                "h".into(),
                "i".into(),
            )
            .await;
        });

        let mut k = 0;
        check_no_leak!("payloads of options and results arrive intact", {
            if k % 2 == 0 {
                many_arguments_cases(
                    Some(42),
                    Err("error".into()),
                    "c".into(),
                    "d".into(),
                    "e".into(),
                    "f".into(),
                    "g".into(),
                    "h".into(),
                )
                .await;
            } else {
                many_arguments_cases(
                    None,
                    Ok(7),
                    "c".into(),
                    "d".into(),
                    "e".into(),
                    "f".into(),
                    "g".into(),
                    "h".into(),
                )
                .await;
            }
            k += 1;
        });

        check_no_leak!("a record in one parameter", {
            one_big_record(BigRecord {
                f0: 0,
                f1: 1,
                f2: 2,
                f3: 3,
                f4: 4,
                f5: 5,
                f6: 6,
                f7: 7,
                f8: 8,
                f9: 9,
                f10: 10,
                f11: 11,
                f12: 12,
                f13: 13,
                f14: 14,
                f15: 15,
                f16: 16,
            })
            .await;
        });

        check_no_leak!("direct parameters need no block", {
            direct_values(
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
                "g".into(),
                8,
            )
            .await;
        });

        if supports_cancel() {
            check_no_leak!("cancelled calls free their arguments", {
                let (tx, rx) = wit_future::new(|| unreachable!());
                let mut call = Box::pin(many_arguments_pending(
                    rx,
                    "a".into(),
                    "b".into(),
                    "c".into(),
                    "d".into(),
                    "e".into(),
                    "f".into(),
                    "g".into(),
                    "h".into(),
                ));
                assert!(call
                    .as_mut()
                    .poll(&mut Context::from_waker(noop_waker_ref()))
                    .is_pending());
                drop(call);
                tx.write(()).await.unwrap_err();
            });
        } else {
            println!("cancelled calls: this component cannot cancel, skipped");
        }
    }
}
