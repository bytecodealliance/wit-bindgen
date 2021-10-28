//! Helper library support for `async` witx functions, used for both

use self::event::{Event, Signal};
use std::cell::{Cell, RefCell};
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

/// Runs the `future` provided to completion, polling the future whenever its
/// waker receives a call to `wake`.
pub fn execute(future: Pin<Box<dyn Future<Output = ()>>>) {
    Task::execute(future)
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Arc<WasmWaker>,
}

impl Task {
    fn execute(future: Pin<Box<dyn Future<Output = ()>>>) {
        Box::new(Task {
            future,
            waker: Arc::new(WasmWaker {
                state: Cell::new(State::Woken),
            }),
        })
        .signal()
    }
}

impl Signal for Task {
    fn signal(mut self: Box<Self>) {
        // First, reset our state to `polling` to indicate that we're actively
        // polling the future that we own.
        let waker = self.waker.clone();
        match waker.state.replace(State::Polling) {
            // This shouldn't be possible since if a waiting event is pending
            // then we shouldn't be woken up to signal.
            State::Waiting(_) => panic!("signaled but event is present"),

            // This also shouldn't be possible since if the previous state were
            // polling then we shouldn't be restarting another round of polling.
            State::Polling => panic!("poll-in-poll"),

            // This is the expected state, which is to say that we should be
            // previously woken with some event having been consumed, which
            // left a `Woken` marker here.
            State::Woken => {}
        }

        // Perform the Rust Dance to poll the future.
        let rust_waker = waker.clone().into();
        let mut cx = Context::from_waker(&rust_waker);
        match self.future.as_mut().poll(&mut cx) {
            // If the future has finished there's nothing else left to do but
            // destroy the future, so we do so here through the dtor for `self`
            // in an early-return.
            Poll::Ready(()) => return,

            // If the future isn't ready then logic below handles the wakeup
            // procedure.
            Poll::Pending => {}
        }

        // Our future isn't ready but we should be scheduled to wait on some
        // event from within the future. Configure the state of the waker
        // after-the-fact to have an interface-types-provided "event" which,
        // when woken, will basically re-invoke this method.
        let event = Event::new(self);
        match waker.state.replace(State::Waiting(event)) {
            // This state shouldn't be possible because we're the only ones
            // inserting a `Waiting` state here, so if something else set that
            // it's highly unexpected.
            State::Waiting(_) => unreachable!(),

            // This is the expected state most of the time where we're replacing
            // the `Polling` state that was configured above. This means we've
            // switched from polling-to-waiting so we can safely return now and
            // wait for our result.
            State::Polling => {}

            // This is a slightly tricky state where we received a `wake()`
            // while we were polling. In this situation we replace the state
            // back to `Woken` and signal the event ourselves.
            State::Woken => {
                let event = match waker.state.replace(State::Woken) {
                    State::Waiting(event) => event,
                    _ => unreachable!(),
                };
                event.signal();
            }
        }
    }
}

/// This is the internals of the `Waker` that's specific to wasm.
///
/// For now this is pretty simple where this maintains a state enum where the
/// main interesting state is an "event" that gets a signal to start re-polling
/// the future. This event-based-wakeup has two consequences:
///
/// * If the `wake()` comes from another Rust coroutine then we'll correctly
///   execute the Rust poll on the original coroutine's context.
/// * If the `wake()` comes from an async import completing then it means the
///   completion callback will do a tiny bit of work to signal the event, and
///   then the real work will happen later when the event's callback is
///   enqueued.
struct WasmWaker {
    state: Cell<State>,
}

enum State {
    Waiting(Event),
    Polling,
    Woken,
}

// These are valid for single-threaded WebAssembly because everything is
// single-threaded and send/sync don't matter much. This module will need
// an alternative implementation for threaded WebAssembly when that comes about
// to host runtimes off-the-web.
#[cfg(not(target_feature = "atomics"))]
unsafe impl Send for WasmWaker {}
#[cfg(not(target_feature = "atomics"))]
unsafe impl Sync for WasmWaker {}

impl Wake for WasmWaker {
    fn wake(self: Arc<Self>) {
        match self.state.replace(State::Woken) {
            // We found a waiting event, yay! Signal that to wake it up and then
            // there's nothing much else for us to do.
            State::Waiting(event) => event.signal(),

            // this `wake` happened during the poll of the future itself, which
            // is ok and the future will consume our `Woken` status when it's
            // done polling.
            State::Polling => {}

            // This is perhaps a concurrent wake where we already woke up the
            // main future. That's ok, we're still in the `Woken` state and it's
            // still someone else's responsibility to manage wakeups at this
            // point.
            State::Woken => {}
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
        // TODO: this oneshot implementation does not correctly handle "hangups"
        // on either the sender or receiver side. This really only works with
        // the exact codegen that we have right now and if it's used for
        // anything else then this implementation needs to be updated (or this
        // should use something off-the-shelf from the ecosystem)
        let inner = Rc::new(OneshotInner {
            state: RefCell::new(OneshotState::Start),
        });
        (
            Oneshot {
                inner: inner.clone(),
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
        let prev = mem::replace(&mut *self.inner.state.borrow_mut(), OneshotState::Done(val));

        match prev {
            // nothing has polled the returned future just yet, so we just
            // filled in the result of the computation. Presumably this will
            // get picked up at some point in the future.
            OneshotState::Start => {}

            // Something was waiting for the result, so we wake the waker
            // here which, for wasm, will likely induce polling immediately.
            OneshotState::Waiting(waker) => waker.wake(),

            // Shouldn't be possible, this is the only closure that writes
            // `Done` and this can only be invoked once. Additionally since
            // `self` exists we shouldn't be closed yet which is only written in
            // `Drop`
            OneshotState::Done(_) => unreachable!(),
        }
    }
}

mod event {
    use std::mem;

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "canonical_abi")]
    extern "C" {
        fn event_new(cb: usize, cbdata: usize) -> u32;
        fn event_signal(handle: u32, arg: u32);
    }

    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn event_new(_: usize, _: usize) -> u32 {
        unreachable!()
    }

    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn event_signal(_: u32, _: u32) {
        unreachable!()
    }

    pub struct Event(u32);

    pub trait Signal {
        fn signal(self: Box<Self>);
    }

    impl Event {
        pub fn new<S>(to_signal: Box<S>) -> Event
        where
            S: Signal,
        {
            unsafe {
                let to_signal = Box::into_raw(to_signal);
                let handle = event_new(signal::<S> as usize, to_signal as usize);
                return Event(handle);
            }

            unsafe extern "C" fn signal<S: Signal>(data: usize, is_drop: u32) {
                let data = Box::from_raw(data as *mut S);
                if is_drop == 0 {
                    data.signal();
                }
            }
        }

        pub fn signal(self) {
            unsafe {
                event_signal(self.0, 0);
                mem::forget(self);
            }
        }
    }

    impl Drop for Event {
        fn drop(&mut self) {
            unsafe {
                event_signal(self.0, 1);
            }
        }
    }
}
