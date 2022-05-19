//! Helper library support for `async` wit functions, used for both

use std::cell::RefCell;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::*;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "canonical_abi")]
extern "C" {
    pub fn async_export_done(ctx: i32, ptr: i32);
}

#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn async_export_done(_ctx: i32, _ptr: i32) {
    panic!("only supported on wasm");
}

struct PollingWaker {
    state: RefCell<State>,
}

enum State {
    Waiting(Pin<Box<dyn Future<Output = ()>>>),
    Polling,
    Woken,
}

// These are valid for single-threaded WebAssembly because everything is
// single-threaded and send/sync don't matter much. This module will need
// an alternative implementation for threaded WebAssembly when that comes about
// to host runtimes off-the-web.
#[cfg(not(target_feature = "atomics"))]
unsafe impl Send for PollingWaker {}
#[cfg(not(target_feature = "atomics"))]
unsafe impl Sync for PollingWaker {}

/// Runs the `future` provided to completion, polling the future whenever its
/// waker receives a call to `wake`.
pub fn execute(future: impl Future<Output = ()> + 'static) {
    let waker = Arc::new(PollingWaker {
        state: RefCell::new(State::Waiting(Box::pin(future))),
    });
    waker.wake()
}

impl Wake for PollingWaker {
    fn wake(self: Arc<Self>) {
        let mut state = self.state.borrow_mut();
        let mut future = match mem::replace(&mut *state, State::Polling) {
            // We are the first wake to come in to wake-up this future. This
            // means that we need to actually poll the future, so leave the
            // `Polling` state in place.
            State::Waiting(future) => future,

            // Otherwise the future is either already polling or it was already
            // woken while it was being polled, in both instances we reset the
            // state back to `Woken` and then we return. This means that the
            // future is owned by some previous stack frame and will drive the
            // future as necessary.
            State::Polling | State::Woken => {
                *state = State::Woken;
                return;
            }
        };
        drop(state);

        // Create the futures waker/context from ourselves, used for polling.
        let waker = self.clone().into();
        let mut cx = Context::from_waker(&waker);
        loop {
            match future.as_mut().poll(&mut cx) {
                // The future is finished! By returning here we destroy the
                // future and release all of its resources.
                Poll::Ready(()) => break,

                // The future has work yet-to-do, so continue below.
                Poll::Pending => {}
            }

            let mut state = self.state.borrow_mut();
            match *state {
                // This means that we were not woken while we were polling and
                // the state is as it was when we took out the future before. By
                // `Pending` being returned at this point we're guaranteed that
                // our waker will be woken up at some point in the future, which
                // will come look at this future again. This means that we
                // simply store our future and return, since this call to `wake`
                // is now finished.
                State::Polling => {
                    *state = State::Waiting(future);
                    break;
                }

                // This means that we received a call to `wake` while we were
                // polling. Ideally we'd enqueue some sort of microtask-tick
                // here or something like that but for now we just loop around
                // and poll again.
                State::Woken => {}

                // This shouldn't be possible since we own the future, and no
                // one else should insert another future here.
                State::Waiting(_) => unreachable!(),
            }
        }
    }
}

pub struct Oneshot<T> {
    inner: Rc<OneshotInner<T>>,
}

pub struct Sender<T> {
    inner: Rc<OneshotInner<T>>,
}

struct OneshotInner<T> {
    state: RefCell<OneshotState<T>>,
}

enum OneshotState<T> {
    Start,
    Waiting(Waker),
    Done(T),
}

impl<T> Oneshot<T> {
    /// Returns a new "oneshot" channel as well as a completion callback.
    pub fn new() -> (Oneshot<T>, Sender<T>) {
        let inner = Rc::new(OneshotInner {
            state: RefCell::new(OneshotState::Start),
        });
        (
            Oneshot {
                inner: Rc::clone(&inner),
            },
            Sender { inner },
        )
    }
}

impl<T> Future for Oneshot<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.inner.state.borrow_mut();
        match mem::replace(&mut *state, OneshotState::Start) {
            OneshotState::Done(t) => Poll::Ready(t),
            OneshotState::Waiting(_) | OneshotState::Start => {
                *state = OneshotState::Waiting(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<T> Sender<T> {
    pub fn into_usize(self) -> usize {
        Rc::into_raw(self.inner) as usize
    }

    pub unsafe fn from_usize(ptr: usize) -> Sender<T> {
        Sender {
            inner: Rc::from_raw(ptr as *const _),
        }
    }

    pub fn send(self, val: T) {
        let mut state = self.inner.state.borrow_mut();
        let prev = mem::replace(&mut *state, OneshotState::Done(val));
        // Must `drop` before the `wake` below because waking may induce
        // polling which would induce another `borrow_mut` which would
        // conflict with this `borrow_mut` otherwise.
        drop(state);

        match prev {
            // nothing has polled the returned future just yet, so we just
            // filled in the result of the computation. Presumably this will
            // get picked up at some point in the future.
            OneshotState::Start => {}

            // Something was waiting for the result, so we wake the waker
            // here which, for wasm, will likely induce polling immediately.
            OneshotState::Waiting(waker) => waker.wake(),

            // Shouldn't be possible, this is the only closure that writes
            // `Done` and this can only be invoked once.
            OneshotState::Done(_) => unreachable!(),
        }
    }
}

impl<T> Drop for OneshotInner<T> {
    fn drop(&mut self) {
        if let OneshotState::Waiting(waker) = &*self.state.borrow() {
            waker.wake_by_ref();
        }
    }
}
