//! Bindings used to manage subtasks, or invocations of imported functions.
//!
//! See `future_support` for some more discussion but the basic idea is the same
//! where we require that everything is passed by ownership to primarily deal
//! with the possibility of leaking futures. By always requiring ownership we
//! can guarantee that even when a future is leaked all its parameters passed to
//! the canonical ABI are additionally leaked with it which should be memory
//! safe.

use crate::async_support::waitable::{WaitableOp, WaitableOperation};
use crate::async_support::{STATUS_RETURNED, STATUS_STARTED, STATUS_STARTING};
use crate::Cleanup;
use std::alloc::Layout;
use std::future::Future;
use std::marker;
use std::num::NonZeroU32;
use std::ptr;

/// Raw operations used to invoke an imported asynchronous function.
///
/// This trait is implemented by generated bindings and is used to implement
/// asynchronous imports.
///
/// # Unsafety
///
/// All operations/constants must be self-consistent for how this module expects
/// them all to be used.
pub unsafe trait Subtask {
    /// The in-memory layout of both parameters and results allocated with
    /// parameters coming first.
    const ABI_LAYOUT: Layout;

    /// The offset, in bytes, from the start of `ABI_LAYOUT` to where the
    /// results will be stored.
    const RESULTS_OFFSET: usize;

    /// The parameters to this task.
    type Params;
    /// The results of this task.
    type Results;

    /// The raw function import using `[async-lower]` and the canonical ABI.
    unsafe fn call_import(params: *mut u8, results: *mut u8) -> u32;

    /// Bindings-generated version of lowering `params` into a heap-allocated
    /// `dst`.
    unsafe fn params_lower(params: Self::Params, dst: *mut u8);

    /// Bindings-generated version of deallocating any lists stored within
    /// `dst`.
    unsafe fn params_dealloc_lists(dst: *mut u8);

    /// Bindings-generated version of lifting the results stored at `src`.
    unsafe fn results_lift(src: *mut u8) -> Self::Results;

    /// Helper function to actually perform this asynchronous call with
    /// `params`.
    fn call(params: Self::Params) -> impl Future<Output = Self::Results>
    where
        Self: Sized,
    {
        WaitableOperation::<SubtaskOps<Self>>::new(Start { params })
    }
}

struct SubtaskOps<T>(marker::PhantomData<T>);

struct Start<T: Subtask> {
    params: T::Params,
}

unsafe impl<T: Subtask> WaitableOp for SubtaskOps<T> {
    type Start = Start<T>;
    type InProgress = InProgress<T>;
    type Result = T::Results;
    type Cancel = ();

    fn start(state: Self::Start) -> (u32, Self::InProgress) {
        unsafe {
            let (ptr_params, cleanup) = Cleanup::new(T::ABI_LAYOUT);
            let ptr_results = ptr_params.add(T::RESULTS_OFFSET);
            T::params_lower(state.params, ptr_params);
            let packed = T::call_import(ptr_params, ptr_results);
            let code = packed & 0xf;
            let subtask = NonZeroU32::new(packed >> 4).map(|handle| SubtaskHandle { handle });
            rtdebug!("<import>({ptr_params:?}, {ptr_results:?}) = ({code:#x}, {subtask:#x?})");

            (
                code,
                InProgress {
                    params_and_results: cleanup,
                    subtask,
                    started: false,
                    _marker: marker::PhantomData,
                },
            )
        }
    }

    fn start_cancelled(_state: Self::Start) -> Self::Cancel {}

    fn in_progress_update(
        mut state: Self::InProgress,
        code: u32,
    ) -> Result<Self::Result, Self::InProgress> {
        match code {
            // Nothing new to do in this state, we're still waiting for the task
            // to start.
            STATUS_STARTING => {
                assert!(!state.started);
                Err(state)
            }

            // Still not done yet, but we can record that this is started and
            // otherwise deallocate lists in the parameters.
            STATUS_STARTED => {
                state.flag_started();
                Err(state)
            }

            STATUS_RETURNED => {
                // Conditionally flag as started if we haven't otherwise
                // explicitly transitioned through `STATUS_STARTED`.
                if !state.started {
                    state.flag_started();
                }

                // Now that our results have been written we can read them.
                //
                // Note that by dropping `state` here we'll both deallocate the
                // params/results storage area as well as the subtask handle
                // itself.
                unsafe { Ok(T::results_lift(state.ptr_results())) }
            }
            other => panic!("unknown code {other:#x}"),
        }
    }

    fn in_progress_waitable(state: &Self::InProgress) -> u32 {
        // This shouldn't get called in the one case this isn't present: when
        // `STATUS_RETURNED` is returned and no waitable is created. That's the
        // `unwrap()` condition here.
        state.subtask.as_ref().unwrap().handle.get()
    }

    fn in_progress_cancel(_: &Self::InProgress) -> u32 {
        // FIXME: plan is to implement cancellation in the canonical ABI in the
        // near future, this will get filled out soon in theory.
        trap_because_of_future_cancel()
    }

    fn result_into_cancel(_result: Self::Result) -> Self::Cancel {
        todo!()
    }
}

#[derive(Debug)]
struct SubtaskHandle {
    handle: NonZeroU32,
}

impl Drop for SubtaskHandle {
    fn drop(&mut self) {
        unsafe {
            subtask_drop(self.handle.get());
        }

        #[cfg(not(target_arch = "wasm32"))]
        unsafe fn subtask_drop(_: u32) {
            unreachable!()
        }

        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "$root")]
        extern "C" {
            #[link_name = "[subtask-drop]"]
            fn subtask_drop(handle: u32);
        }
    }
}

struct InProgress<T: Subtask> {
    params_and_results: Option<Cleanup>,
    started: bool,
    subtask: Option<SubtaskHandle>,
    _marker: marker::PhantomData<T>,
}

impl<T: Subtask> InProgress<T> {
    fn flag_started(&mut self) {
        assert!(!self.started);
        self.started = true;

        // SAFETY: the initial entrypoint of `call` requires that the vtable is
        // setup correctly and we're obeying the invariants of the vtable,
        // deallocating lists in an allocation that we exclusively own.
        unsafe {
            T::params_dealloc_lists(self.ptr_params());
        }
    }

    fn ptr_params(&self) -> *mut u8 {
        self.params_and_results
            .as_ref()
            .map(|c| c.ptr.as_ptr())
            .unwrap_or(ptr::null_mut())
    }

    fn ptr_results(&self) -> *mut u8 {
        // SAFETY: the `T` trait has unsafely promised us that the offset is
        // in-bounds of the allocation layout.
        unsafe { self.ptr_params().add(T::RESULTS_OFFSET) }
    }
}

#[cold]
fn trap_because_of_future_cancel() -> ! {
    panic!(
        "an imported function is being dropped/cancelled before being fully \
         awaited, but that is not sound at this time so the program is going \
         to be aborted; for more information see \
         https://github.com/bytecodealliance/wit-bindgen/issues/1175"
    )
}
