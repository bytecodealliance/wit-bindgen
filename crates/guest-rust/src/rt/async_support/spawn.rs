// TODO: Switch to interior mutability (e.g. use Mutexes or thread-local
// RefCells) and remove this, since even in single-threaded mode `static mut`
// references can be a hazard due to recursive access.
#![allow(static_mut_refs)]

use crate::rt::async_support::BoxFuture;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures::channel::oneshot;
use futures::future::{AbortHandle, Abortable, Aborted};
use futures::stream::{FuturesUnordered, StreamExt};

/// Any newly-deferred work queued by calls to the `spawn` function while
/// polling the current task.
static mut SPAWNED: Vec<BoxFuture> = Vec::new();

#[derive(Default)]
pub struct Tasks<'a> {
    tasks: FuturesUnordered<BoxFuture<'a>>,
}

impl<'a> Tasks<'a> {
    pub fn new(root: BoxFuture<'a>) -> Tasks<'a> {
        Tasks {
            tasks: [root].into_iter().collect(),
        }
    }

    pub fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            // Perform some work by seeing what's next in this
            // `FuturesUnordered`. Afterwards check the set of spawned tasks
            // and, if any, add them to our set of tasks being done.
            let poll = self.tasks.poll_next_unpin(cx);
            let spawned = unsafe {
                if SPAWNED.is_empty() {
                    false
                } else {
                    self.tasks.extend(SPAWNED.drain(..));
                    true
                }
            };
            match poll {
                // If no tasks were ready, and if we didn't spawn any work,
                // then there's nothing left to do so return pending.
                //
                // If no tasks were ready, and if we spawned some work, then
                // turn the loop again to register interest in the work and
                // ensure that it's not forgotten about.
                Poll::Pending => {
                    if !spawned {
                        return Poll::Pending;
                    }
                }

                // If our set of tasks is empty it shouldn't be possible to have
                // spawned anything, and return saying that we're done.
                Poll::Ready(None) => {
                    assert!(!spawned);
                    return Poll::Ready(());
                }

                // If a task finished, then turn the loop again to see if there
                // are any other completed tasks. This also serves double-duty
                // to ensure that we look at everything in our set of tasks
                // before concluding that we're finished.
                Poll::Ready(Some(())) => {}
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Spawn the provided `future` to get executed concurrently with the
/// currently-running async computation.
///
/// This API is somewhat similar to `tokio::task::spawn` for example but has a
/// number of limitations to be aware of. If possible it's recommended to avoid
/// this, but it can be convenient if these limitations do not apply to you:
///
/// * Spawned tasks do not work when the version of the `wit-bindgen` crate
///   managing the export bindings is different from the version of this crate.
///   To work correctly the `spawn` function and export executor must be at
///   exactly the same version. Given the major-version-breaking nature of
///   `wit-bindgen` this is not always easy to rely on. This is tracked in
///   [#1305].
///
/// * Spawned tasks do not outlive the scope of the async computation they are
///   spawned within. For example with an async export function spawned tasks
///   will be polled within the context of that component-model async task. For
///   computations executing within a [`block_on`] call, however, the spawned
///   tasks will be executed within that scope. This notably means that for
///   [`block_on`] spawned tasks will prevent the [`block_on`] function from
///   returning, even if a value is available to return. If `spawn_local` is
///   called within a component-model async task which is then terminated (e.g.
///   by the host) before the future resolves, awating the `Task` will return
///   `None`.
///
/// * The task spawned here is executed *concurrently*, not in *parallel*. This
///   means that while one future is being polled no other future can be polled
///   at the same time. This is similar to a single-thread executor in Tokio.
///
/// With these restrictions in mind this can be used to express
/// execution-after-returning in the component model. For example once an
/// exported async function has produced a value this can be used to continue to
/// execute some more code before the component model async task exits.
///
/// # Cancellation
///
/// Dropping the resulting [`Task`] will cancel the spawned future. [`Task::detach`] will
/// allow the future to continue running in the background and [`Task::cancel`] will
/// explicitly wait for the cancelation to complete.
///
/// [`block_on`]: crate::block_on
/// [#1305]: https://github.com/bytecodealliance/wit-bindgen/issues/1305
pub fn spawn_local<T: 'static>(future: impl Future<Output = T> + 'static) -> Task<T> {
    let (sender, receiver) = oneshot::channel();
    let (abort, registration) = AbortHandle::new_pair();
    unsafe {
        SPAWNED.push(Box::pin(async move {
            let _ = sender.send(Abortable::new(future, registration).await);
        }));
    }
    Task {
        receiver,
        abort,
        cancel_on_drop: true,
    }
}

/// A handle to a spawned task which can be awaited for its result.
///
/// Dropping this handle cancels the task. To drop the handle without cancelling
/// the task, call [`detach`](Self::detach). Awaiting the handle returns `None`
/// if the task was cancelled or otherwise terminated without producing a
/// result.
#[must_use = "dropping the handle cancels the spawned task"]
pub struct Task<T> {
    receiver: oneshot::Receiver<Result<T, Aborted>>,
    abort: AbortHandle,
    cancel_on_drop: bool,
}

impl<T> Task<T> {
    /// Cancels the spawned task and waits for cancellation to complete.
    ///
    /// This returns the task's output if it completed before it could be
    /// cancelled, or `None` if it was cancelled or otherwise terminated.
    pub async fn cancel(mut self) -> Option<T> {
        self.abort.abort();
        self.cancel_on_drop = false;
        match (&mut self.receiver).await {
            Ok(Ok(result)) => Some(result),
            Ok(Err(_)) => None,
            Err(_) => None,
        }
    }

    /// Detaches the spawned task, allowing it to continue in the background.
    pub fn detach(mut self) {
        self.cancel_on_drop = false;
    }
}

impl<T> Future for Task<T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(Ok(result))) => Poll::Ready(Some(result)),
            Poll::Ready(Ok(Err(_)) | Err(_)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if self.cancel_on_drop {
            self.abort.abort();
        }
    }
}
