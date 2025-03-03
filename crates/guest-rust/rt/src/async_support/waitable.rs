//! Generic support for "any waitable" and performing asynchronous operations on
//! that waitable.

use super::results;
use std::marker;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct WaitableOperation<S: WaitableOp> {
    state: WaitableOperationState<S>,
    completion_status: CompletionStatus,
}

struct CompletionStatus {
    code: Option<u32>,
    _pinned: marker::PhantomPinned,
}

/// Helper trait to be used with `WaitableOperation` to assist with machinery
/// necessary to track in-flight reads/writes on futures.
pub unsafe trait WaitableOp {
    /// Initial state of this operation, used to kick off the actual component
    /// model operation and transition to `InProgress`.
    type Start;

    /// Intermediate state of this operation when the component model is
    /// involved but it hasn't resolved just yet.
    type InProgress;

    /// Result type of this operation.
    type Result;

    /// Result of when this operation is cancelled.
    type Cancel;

    /// Starts the async operation.
    ///
    /// This method will actually call `{future,stream}.{read,write}` with
    /// `state` provided. The return code of the intrinsic is returned here
    /// along with the `InProgress` state.
    fn start(state: Self::Start) -> (u32, Self::InProgress);

    /// Conversion from the "start" state to the "cancel" result, needed when an
    /// operation is cancelled before it's started.
    fn start_cancel(state: Self::Start) -> Self::Cancel;

    /// Completion callback for when an in-progress operation has completed
    /// successfully after transferring `amt` items.
    fn in_progress_complete(state: Self::InProgress, amt: u32) -> Self::Result;

    /// Completion callback for when an in-progress operation has completed
    /// without actually transferring anything because the other end has closed.
    fn in_progress_closed(state: Self::InProgress) -> Self::Result;

    /// Acquires the component-model `waitable` index that the `InProgress`
    /// state is waiting on.
    fn in_progress_waitable(state: &Self::InProgress) -> u32;

    /// Initiates a request for cancellation of this operation. Returns the
    /// status code returned by the `{future,stream}.cancel-{read,write}`
    /// intrinsic.
    fn in_progress_cancel(state: &Self::InProgress) -> u32;

    /// Completion callback for when an operation was canceled.
    ///
    /// This is invoked after `in_progress_cancel` is used and the returned
    /// status code indicates that the operation was indeed cancelled and didn't
    /// racily return some other result.
    fn in_progress_canceled(state: Self::InProgress) -> Self::Result;

    /// Converts a "completion result" into a "cancel result". This is necessary
    /// when an in-progress operation is cancelled so the in-progress result is
    /// first acquired and then transitioned to a cancel request.
    fn result_into_cancel(result: Self::Result) -> Self::Cancel;
}

enum WaitableOperationState<S: WaitableOp> {
    Start(S::Start),
    InProgress(S::InProgress),
    Done,
}

impl<S> WaitableOperation<S>
where
    S: WaitableOp,
{
    /// Creates a new operation in the initial state.
    pub fn new(state: S::Start) -> WaitableOperation<S> {
        WaitableOperation {
            state: WaitableOperationState::Start(state),
            completion_status: CompletionStatus {
                code: None,
                _pinned: marker::PhantomPinned,
            },
        }
    }

    fn pin_project(
        self: Pin<&mut Self>,
    ) -> (&mut WaitableOperationState<S>, Pin<&mut CompletionStatus>) {
        // SAFETY: this is the one method used to project from `Pin<&mut Self>`
        // to the fields, and the contract we're deciding on is that
        // `state` is never pinned but the `CompletionStatus` is. That's used
        // to share a raw pointer with the completion callback with
        // respect to `Option<u32>` internally.
        unsafe {
            let me = self.get_unchecked_mut();
            (&mut me.state, Pin::new_unchecked(&mut me.completion_status))
        }
    }

    /// Registers a completion of `waitable` within the current task's future to:
    ///
    /// * Fill in `completion_status` with the result of a completion event.
    /// * Call `cx.waker().wake()`.
    pub fn register_waker(self: Pin<&mut Self>, waitable: u32, cx: &mut Context) {
        let (_, completion_status) = self.pin_project();
        unsafe {
            (*super::CURRENT).add_waitable(waitable);
            (*super::CURRENT)
                .wakers
                .insert(waitable, (cx.waker().clone(), completion_status.code_mut()));
        }
    }

    /// Deregisters the corresponding `register_waker` within the current task
    /// for the `waitable` passed here.
    ///
    /// This relinquishes control of the original `completion_status` pointer
    /// passed to `register_waker` after this call has completed.
    pub fn unregister_waker(self: Pin<&mut Self>, waitable: u32) {
        unsafe {
            (*super::CURRENT).remove_waitable(waitable);
            let prev = (*super::CURRENT).wakers.remove(&waitable);
            assert!(prev.is_some());
        }
    }

    /// Polls this operation to see if it has completed yet.
    ///
    /// This is intended to be used within `Future::poll`.
    pub fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<S::Result> {
        use WaitableOperationState::*;

        let (state, completion_status) = self.as_mut().pin_project();

        // First up, determine the completion status, if any, that's available.
        let optional_code = match state {
            // If this operation hasn't actually started yet then now's the
            // time to start it.
            Start(_) => {
                let Start(s) = mem::replace(state, Done) else {
                    unreachable!()
                };
                let (code, s) = S::start(s);
                *state = InProgress(s);
                match code {
                    // The operation is blocked, meaning it didn't complete.
                    //
                    // We've already transitioned to the in-progress state so
                    // all that's left to do is indicate that we don't have a
                    // return code at this time.
                    results::BLOCKED => None,

                    // This operation completed immediately.
                    //
                    // As above we're in the in-progress state, so defer what to do
                    // with this code to down below.
                    other => Some(other),
                }
            }

            // This operation was previously queued so we're just waiting on
            // the completion to come in. Read the completion status and
            // interpret it down below.
            //
            // Note that it's the responsibility of the completion callback at
            // the ABI level that we install to fill in this pointer, e.g. it's
            // part of the `register_waker` contract.
            InProgress(_) => completion_status.code,

            // This write has already completed, it's a Rust-level API violation
            // to call this function again.
            Done => panic!("cannot re-poll after operation completes"),
        };

        self.poll_complete_with_code(Some(cx), optional_code)
    }

    /// After acquiring the current return of this operation in `optional_code`,
    /// figures out what to do with it.
    ///
    /// The `cx` argument is optional to do nothing in the case that
    /// `optional_code` is not present.
    fn poll_complete_with_code(
        mut self: Pin<&mut Self>,
        cx: Option<&mut Context>,
        optional_code: Option<u32>,
    ) -> Poll<S::Result> {
        use WaitableOperationState::*;

        let (state, _completion_status) = self.as_mut().pin_project();
        let in_progress = match state {
            InProgress(s) => s,
            // programmer error if this is called in the wrong state.
            _ => unreachable!(),
        };

        let code = match optional_code {
            Some(code) => code,

            // The operation is still in progress.
            //
            // Register the `cx.waker()` to get notified when `writer.handle`
            // receives its completion.
            None => {
                if let Some(cx) = cx {
                    let handle = S::in_progress_waitable(in_progress);
                    self.register_waker(handle, cx);
                }
                return Poll::Pending;
            }
        };

        // After this point we're guaranteed the operation has completed, so
        // it's time to interpret the result and return.
        let InProgress(in_progress) = mem::replace(state, Done) else {
            unreachable!()
        };

        match code {
            // The other end has closed or the operation was cancelled and the
            // operation did not complete. See what `S` has to say about that.
            results::CLOSED => Poll::Ready(S::in_progress_closed(in_progress)),
            results::CANCELED => Poll::Ready(S::in_progress_canceled(in_progress)),

            // This operation has completed, transferring `n` units of memory.
            //
            // Forward this information to `S` and see what it has to say about
            // that.
            n => Poll::Ready(S::in_progress_complete(in_progress, n)),
        }
    }

    /// Cancels the in-flight operation, if it's still in-flight, and sees what
    /// happened.
    ///
    /// Defers to `S` how to communicate the current status through the
    /// cancellation type.
    ///
    /// # Panics
    ///
    /// Panics if the operation has already been completed via `poll_complete`
    /// above.
    /// Panics if this method is called twice.
    pub fn cancel(mut self: Pin<&mut Self>) -> S::Cancel {
        use WaitableOperationState::*;

        let (state, _) = self.as_mut().pin_project();
        let in_progress = match state {
            // This operation was never actually started, so there's no need to
            // cancel anything, just pull out the value and return it.
            Start(_) => {
                let Start(s) = mem::replace(state, Done) else {
                    unreachable!()
                };
                return S::start_cancel(s);
            }

            // This operation is actively in progress, fall through to below.
            InProgress(s) => s,

            // This operation was already completed after a `poll_complete`
            // above advanced to the `Done` state, or this was cancelled twice.
            // In such situations this is a programmer error to call this
            // method, so panic.
            Done => panic!("cannot cancel operation after completing it"),
        };

        // This operation is currently actively in progress after being queued
        // up in the past. In this situation we need to call
        // `{future,stream}.cancel-{read,write}`. First ensure that our
        // exported task's state is no longer interested in the write handle
        // here, so unregister that. Next if a completion hasn't already come
        // in due to some race then perform the actual cancellation here.
        let waitable = S::in_progress_waitable(in_progress);
        self.as_mut().unregister_waker(waitable);
        let (InProgress(in_progress), mut completion_status) = self.as_mut().pin_project() else {
            unreachable!()
        };
        if completion_status.code.is_none() {
            *completion_status.as_mut().code_mut() = Some(S::in_progress_cancel(in_progress));
        }

        // Now that we're guaranteed to have a completion status, pass that
        // through to "interpret the result".
        let code = completion_status.code.unwrap();
        match self.poll_complete_with_code(None, Some(code)) {
            // Leave it up to `S` to interpret the completion result as a
            // cancellation result.
            Poll::Ready(result) => S::result_into_cancel(result),

            // Should not be reachable as we always pass `Some(code)`.
            Poll::Pending => unreachable!(),
        }
    }
}

impl<S: WaitableOp> Drop for WaitableOperation<S> {
    fn drop(&mut self) {
        // SAFETY: we're in the destructor here so the value `self` is about
        // to go away and we can guarantee we're not moving out of it.
        let mut pin = unsafe { Pin::new_unchecked(self) };

        let (state, _) = pin.as_mut().pin_project();

        // If this operation has already completed then skip cancellation,
        // otherwise it's our job to cancel anything in-flight.
        if let WaitableOperationState::Done = state {
            return;
        }
        pin.cancel();
    }
}

impl CompletionStatus {
    fn code_mut(self: Pin<&mut Self>) -> &mut Option<u32> {
        unsafe { &mut self.get_unchecked_mut().code }
    }
}
