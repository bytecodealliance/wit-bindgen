use crate::rt::async_support::BoxFuture;
use alloc::boxed::Box;
use core::future::Future;
use core::task::{Context, Poll};

#[derive(Default)]
pub struct Tasks<'a> {
    future: Option<BoxFuture<'a>>,
}

impl<'a> Tasks<'a> {
    pub fn new(root: impl Future<Output = ()> + 'a) -> Tasks<'a> {
        Tasks {
            future: Some(Box::pin(root)),
        }
    }

    pub fn cancel_root(&mut self) {
        self.future = None;
    }

    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(future) = self.future.as_mut() {
            if future.as_mut().poll(cx).is_ready() {
                self.future = None;
            }
        }
        if self.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn is_empty(&self) -> bool {
        self.future.is_none()
    }
}
