//! Definition of the "C ABI" of how imported functions interact with exported
//! tasks.
//!
//! Ok this crate is written in Rust, why in the world does this exist? This
//! comment is intended to explain this rationale but the tl;dr; is we want
//! this to work:
//!
//! * Within a single component ...
//! * One rust crate uses `wit-bindgen 0.A.0` to generate an exported function.
//! * One rust crate uses `wit-bindgen 0.B.0` to bind an imported function.
//! * The two crates are connected in the application with
//!   `std::future::Future`.
//!
//! Without this module this situation won't work because 0.A.0 has no
//! knowledge of 0.B.0 meaning that 0.B.0 has no means of inserting a `waitable`
//! into the `waitable-set` managed by 0.A.0's export.
//!
//! To solve this problem the long-term intention is that something will live
//! in `wasi-libc` itself, but in the meantime it's living "somewhere" within
//! `wit-bindgen 0.*.0`. Specifically all `wit-bindgen` versions will
//! reference, via C linkage, a single function which is used to manipulate a
//! single pointer in linear memory. This pointer is a `wasip3_task` structure
//! which has all the various fields to use it.
//!
//! The `wasip3_task_set` symbol is itself defined in C inside of the
//! `src/wit_bindgen_cabi.c` file at this time, specifically because it's
//! annotated with `__weak__` meaning that any definition of it suffices. This
//! isn't possible to define in stable Rust (specifically `__weak__`).
//!
//! Once `wasip3_task_set` is defined everything then operates via indirection,
//! aka based off the returned pointer. The intention is that exported functions
//! will set this (it's sort of like an executor) and then imported functions
//! will all use this as the source of registering waitables. In the end that
//! means that it's possible to share types with `std::future::Future` that
//! are backed at the ABI level with this "channel".
//!
//! In the future it's hoped that this can move into `wasi-libc` itself, or if
//! `wasi-libc` provides something else that would be prioritized over this.
//! For now this is basically an affordance that we're going to be frequently
//! releaseing new major versions of `wit-bindgen` and we don't want to force
//! applications to all be using the exact same version of the bindings
//! generator and async bindings.
//!
//! Additionally for now this file is serving as documentation of this
//! interface.
//!
//! # Revisions
//!
//! This interface is intended to be evolvable over time if needed. Notably the
//! original task structure, `wasip3_task`, has a `version` field where certain
//! version levels imply the existence of certain fields. The historical
//! revisions are:
//!
//! ### V1
//!
//! This was the original version. This is the original specification of
//! `wasip3_task_set` and `wasip3_task`.
//!
//! ### V2
//!
//! This was added 2026-06-17 in response to #1618. This added
//! `wasip3_task_v2` and `wasip3_task_vtable`. This version enables cloning a
//! task to create a strong reference to it independent of the stack lifetime
//! that `wasip3_task_set` is required to uphold. This necessitated introducing
//! `clone` and `drop` callbacks to manage the lifetime of this reference.
//! While doing this everything was moved into a vtable structure instead of
//! inline in `wasip3_task` to make it easier to add more function pointers
//! in the future if necessary.

use core::ffi::c_void;

extern_wasm! {
    unsafe extern "C" {
        /// Sets the global task pointer to `ptr` provided. Returns the previous
        /// value.
        ///
        /// This function acts as a dual getter and a setter. To get the
        /// current task pointer a dummy `ptr` can be provided (e.g. NULL) and then
        /// it's passed back when you're done working with it. When setting the
        /// current task pointer it's recommended to call this and then call it
        /// again with the previous value when the tasks's work is done.
        ///
        /// For executors they need to ensure that the `ptr` passed in lives for
        /// the entire lifetime of the component model task.
        pub fn wasip3_task_set(ptr: *mut wasip3_task) -> *mut wasip3_task;
    }
}

/// The first version of `wasip3_task` which implies the existence of the
/// fields `ptr`, `waitable_register`, and `waitable_unregister`.
pub const WASIP3_TASK_V1: u32 = 1;
pub const WASIP3_TASK_V2: u32 = 2;

/// Indirect "vtable" used to connect imported functions and exported tasks.
/// Executors (e.g. exported functions) define and manage this while imports
/// use it.
#[repr(C)]
pub struct wasip3_task {
    /// Currently `WASIP3_TASK_V1`. Indicates what fields are present next
    /// depending on the version here.
    pub version: u32,

    /// Private pointer owned by the `wasip3_task` itself, passed to callbacks
    /// below as the first argument.
    pub ptr: *mut c_void,

    /// See `wasip3_task_vtable::waitable_register`.
    pub waitable_register: unsafe extern "C" fn(
        ptr: *mut c_void,
        waitable: u32,
        callback: unsafe extern "C" fn(callback_ptr: *mut c_void, code: u32),
        callback_ptr: *mut c_void,
    ) -> *mut c_void,

    /// See `wasip3_task_vtable::waitable_unregister`.
    pub waitable_unregister: unsafe extern "C" fn(ptr: *mut c_void, waitable: u32) -> *mut c_void,
}

unsafe impl Send for wasip3_task {}
unsafe impl Sync for wasip3_task {}

/// Representation when `wasip3_task::version` is `WASIP3_TASK_V2`.
#[repr(C)]
pub struct wasip3_task_v2 {
    /// The original task structure.
    pub v1: wasip3_task,

    /// An always-valid pointer to a list of function pointers, described
    /// below.
    pub vtable: &'static wasip3_task_vtable,
}

/// Function pointer operations that can operate on `wasip3_task::ptr`.
///
/// This was introduced in the "v2" ABI and is a member of `wasip3_task_v2`.
#[repr(C)]
pub struct wasip3_task_vtable {
    /// Register a new `waitable` for this exported task.
    ///
    /// This exported task will add `waitable` to its `waitable-set`. When it
    /// becomes ready then `callback` will be invoked with the ready code as
    /// well as the `callback_ptr` provided.
    ///
    /// If `waitable` was previously registered with this task then the
    /// previous `callback_ptr` is returned. Otherwise `NULL` is returned.
    ///
    /// It's the caller's responsibility to ensure that `callback_ptr` is valid
    /// until `callback` is invoked, `waitable_unregister` is invoked, or
    /// `waitable_register` is called again to overwrite the value.
    pub waitable_register: unsafe extern "C" fn(
        ptr: *mut c_void,
        waitable: u32,
        callback: unsafe extern "C" fn(callback_ptr: *mut c_void, code: u32),
        callback_ptr: *mut c_void,
    ) -> *mut c_void,

    /// Removes the `waitable` from this task's `waitable-set`.
    ///
    /// Returns the `callback_ptr` passed to `waitable_register` if present, or
    /// `NULL` if it's not present.
    pub waitable_unregister: unsafe extern "C" fn(ptr: *mut c_void, waitable: u32) -> *mut c_void,

    /// Clones this task's pointer to create a separately owned pointer which
    /// can be persisted outside the stack frame that this is being used
    /// within.
    ///
    /// Cloned values must be dropped/deallocated with `drop` below.
    pub clone: unsafe extern "C" fn(ptr: *mut c_void) -> *mut c_void,

    /// Drops and deallocates the provided pointer previously created by a
    /// call to the `clone` callback above.
    ///
    /// This must not be called on the `ptr` value within `wasip3_task::ptr` as
    /// that's not managed with this lifetime.
    pub drop: unsafe extern "C" fn(ptr: *mut c_void),
}
