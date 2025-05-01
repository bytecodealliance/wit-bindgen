include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::a::b::i::Guest for Component {
    async fn f() {
        for _ in 0..10 {
            yield_().await;
        }
    }
}

async fn yield_() {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Default)]
    struct Yield {
        yielded: bool,
    }

    impl Future for Yield {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                context.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    Yield::default().await;
}
