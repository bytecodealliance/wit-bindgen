//! Runtime support for `future<T>` in the component model.
//!
//! TODO:
//!
//! * leaking requires owned values
//! * owned values means we can't return back sent items
//! * intimately used in implementation details.

use {
    super::waitable::{WaitableOp, WaitableOperation},
    crate::Cleanup,
    std::{
        alloc::Layout,
        fmt,
        future::{Future, IntoFuture},
        marker,
        pin::Pin,
        ptr,
        sync::atomic::{AtomicU32, Ordering::Relaxed},
        task::{Context, Poll},
    },
};

#[doc(hidden)]
pub struct FutureVtable<T> {
    pub layout: Layout,
    pub lower: unsafe fn(value: T, dst: *mut u8),
    pub dealloc_lists: unsafe fn(dst: *mut u8),
    pub lift: unsafe fn(dst: *mut u8) -> T,
    pub start_write: unsafe extern "C" fn(future: u32, val: *const u8) -> u32,
    pub start_read: unsafe extern "C" fn(future: u32, val: *mut u8) -> u32,
    pub cancel_write: unsafe extern "C" fn(future: u32) -> u32,
    pub cancel_read: unsafe extern "C" fn(future: u32) -> u32,
    pub close_writable: unsafe extern "C" fn(future: u32),
    pub close_readable: unsafe extern "C" fn(future: u32),
    pub new: unsafe extern "C" fn() -> u64,
}

/// Helper function to create a new read/write pair for a component model
/// future.
pub unsafe fn future_new<T>(
    vtable: &'static FutureVtable<T>,
) -> (FutureWriter<T>, FutureReader<T>) {
    unsafe {
        let handles = (vtable.new)();
        (
            FutureWriter::new(handles as u32, vtable),
            FutureReader::new((handles >> 32) as u32, vtable),
        )
    }
}

/// Represents the writable end of a Component Model `future`.
pub struct FutureWriter<T: 'static> {
    handle: u32,
    vtable: &'static FutureVtable<T>,
}

impl<T> FutureWriter<T> {
    #[doc(hidden)]
    pub unsafe fn new(handle: u32, vtable: &'static FutureVtable<T>) -> Self {
        Self { handle, vtable }
    }

    /// Write the specified `value` to this `future`.
    pub fn write(self, value: T) -> FutureWrite<T> {
        FutureWrite {
            op: WaitableOperation::new((self, value)),
        }
    }
}

impl<T> fmt::Debug for FutureWriter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureWriter")
            .field("handle", &self.handle)
            .finish()
    }
}

impl<T> Drop for FutureWriter<T> {
    fn drop(&mut self) {
        unsafe {
            (self.vtable.close_writable)(self.handle);
        }
    }
}

/// Represents a write operation which may be canceled prior to completion.
pub struct FutureWrite<T: 'static> {
    op: WaitableOperation<FutureWriteOp<T>>,
}

struct FutureWriteOp<T>(marker::PhantomData<T>);

enum WriteComplete<T> {
    Written,
    Closed(T),
    Cancelled(T),
}

unsafe impl<T> WaitableOp for FutureWriteOp<T>
where
    T: 'static,
{
    type Start = (FutureWriter<T>, T);
    type InProgress = (FutureWriter<T>, Option<Cleanup>);
    type Result = (WriteComplete<T>, FutureWriter<T>);
    type Cancel = Result<(), FutureWriteCancel<T>>;

    fn start((writer, value): Self::Start) -> (u32, Self::InProgress) {
        // TODO: it should be safe to store the lower-destination in
        // `WaitableOperation` using `Pin` memory and such, but that would
        // require some type-level trickery to get a correctly-sized value
        // plumbed all the way to here. For now just dynamically allocate it and
        // leave the optimization of leaving out this dynamic allocation to the
        // future.
        //
        // In lieu of that a dedicated location on the heap is created for the
        // lowering, and then `value`, as an owned value, is lowered into this
        // pointer to initialize it.
        let (ptr, cleanup) = Cleanup::new(writer.vtable.layout);
        // SAFETY: `ptr` is allocated with `vtable.layout` and should be
        // safe to use here.
        let code = unsafe {
            (writer.vtable.lower)(value, ptr);
            (writer.vtable.start_write)(writer.handle, ptr)
        };
        (code, (writer, cleanup))
    }

    fn start_cancel((writer, value): Self::Start) -> Self::Cancel {
        Err(FutureWriteCancel::Cancelled(value, writer))
    }

    /// This write has completed.
    ///
    /// Here we need to clean up our allocations. The `ptr` exclusively owns all
    /// of the value being sent and we notably need to cleanup the transitive
    /// list allocations present in this pointer. Use `dealloc_lists` for that
    /// (effectively a post-return lookalike).
    ///
    /// Afterwards the `cleanup` itself is naturally dropped and cleaned up.
    fn in_progress_complete((writer, cleanup): Self::InProgress, amt: u32) -> Self::Result {
        assert_eq!(amt, 1);
        let ptr = cleanup
            .as_ref()
            .map(|c| c.ptr.as_ptr())
            .unwrap_or(ptr::null_mut());

        // SAFETY: we're the ones managing `ptr` so we know it's safe to
        // pass here.
        unsafe {
            (writer.vtable.dealloc_lists)(ptr);
        }
        (WriteComplete::Written, writer)
    }

    /// The other end has closed its end.
    ///
    /// The value was not received by the other end so `ptr` still has all of
    /// its resources intact. Use `lift` to construct a new instance of `T`
    /// which takes ownership of pointers and resources and such. The allocation
    /// of `ptr` is then cleaned up naturally when `cleanup` goes out of scope.
    fn in_progress_closed((writer, cleanup): Self::InProgress) -> Self::Result {
        let ptr = cleanup
            .as_ref()
            .map(|c| c.ptr.as_ptr())
            .unwrap_or(ptr::null_mut());
        // SAFETY: we're the ones managing `ptr` so we know it's safe to
        // pass here.
        let value = unsafe { (writer.vtable.lift)(ptr) };
        (WriteComplete::Closed(value), writer)
    }

    fn in_progress_waitable((writer, _): &Self::InProgress) -> u32 {
        writer.handle
    }

    fn in_progress_cancel((writer, _): &Self::InProgress) -> u32 {
        // SAFETY: we're managing `writer` and all the various operational bits,
        // so this relies on `WaitableOperation` being safe.
        unsafe { (writer.vtable.cancel_write)(writer.handle) }
    }

    fn in_progress_canceled(state: Self::InProgress) -> Self::Result {
        match Self::in_progress_closed(state) {
            (WriteComplete::Closed(value), writer) => (WriteComplete::Cancelled(value), writer),
            _ => unreachable!(),
        }
    }

    fn result_into_cancel((result, writer): Self::Result) -> Self::Cancel {
        match result {
            // The value was actually sent, meaning we can't yield back the
            // future nor the value.
            WriteComplete::Written => Ok(()),

            // The value was not sent because the other end either hung up or we
            // successfully canceled. In both cases return back the value here
            // with the writer.
            WriteComplete::Closed(val) => Err(FutureWriteCancel::Closed(val)),
            WriteComplete::Cancelled(val) => Err(FutureWriteCancel::Cancelled(val, writer)),
        }
    }
}

impl<T: 'static> Future for FutureWrite<T> {
    type Output = Result<(), FutureWriteError<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pin_project()
            .poll_complete(cx)
            .map(|(result, _writer)| match result {
                WriteComplete::Written => Ok(()),
                WriteComplete::Closed(value) | WriteComplete::Cancelled(value) => {
                    Err(FutureWriteError { value })
                }
            })
    }
}

impl<T: 'static> FutureWrite<T> {
    fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<FutureWriteOp<T>>> {
        // SAFETY: we've chosen that when `Self` is pinned that it translates to
        // always pinning the inner field, so that's codified here.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
    }

    /// Cancel this write if it hasn't already completed.
    ///
    /// This method can be used to cancel a write-in-progress and re-acquire
    /// the writer and the value being sent. If the write operation has already
    /// succeeded racily then `None` is returned and the write completed.
    ///
    /// Possible return values are:
    ///
    /// * `Ok(())` - the pending write completed before cancellation went
    ///   through meaning that the original message was actually sent.
    /// * `Err(FutureWriteCancel::Closed(v))` - the pending write did not complete
    ///   because the other end was closed before receiving the value. The value
    ///   is provided back here as part of the error.
    /// * `Err(FutureWriteCancel::Cancelled(v, writer))` - the pending write was
    ///   cancelled. The value `v` is returned back and the `writer` is returned
    ///   as well to resume a write in the future if desired.
    ///
    /// Note that if this method is called after the write was already cancelled
    /// then `Ok(())` will be returned.
    ///
    /// # Panics
    ///
    /// Panics if the operation has already been completed via `Future::poll`,
    /// or if this method is called twice.
    pub fn cancel(self: Pin<&mut Self>) -> Result<(), FutureWriteCancel<T>> {
        self.pin_project().cancel()
    }
}

/// Error type in the result of [`FutureWrite`], or the error type that is a result of
/// a failure to write a future.
pub struct FutureWriteError<T> {
    /// The value that could not be sent.
    pub value: T,
}

impl<T> fmt::Debug for FutureWriteError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureWriteError").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for FutureWriteError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "read end closed".fmt(f)
    }
}

impl<T> std::error::Error for FutureWriteError<T> {}

/// Error type in the result of [`FutureWrite::cancel`], or the error type that is a
/// result of cancelling a pending write.
pub enum FutureWriteCancel<T: 'static> {
    /// The other end was closed before cancellation happened.
    ///
    /// In this case the original value is returned back to the caller but the
    /// writer itself is not longer accessible as it's no longer usable.
    Closed(T),

    /// The pending write was successfully cancelled and the value being written
    /// is returned along with the writer to resume again in the future if
    /// necessary.
    Cancelled(T, FutureWriter<T>),
}

impl<T> fmt::Debug for FutureWriteCancel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureWriteCancel").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for FutureWriteCancel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FutureWriteCancel::Closed(_) => "read end closed".fmt(f),
            FutureWriteCancel::Cancelled(..) => "write cancelled".fmt(f),
        }
    }
}

impl<T> std::error::Error for FutureWriteCancel<T> {}

/// Represents the readable end of a Component Model `future`.
pub struct FutureReader<T: 'static> {
    handle: AtomicU32,
    vtable: &'static FutureVtable<T>,
}

impl<T> fmt::Debug for FutureReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureReader")
            .field("handle", &self.handle)
            .finish()
    }
}

impl<T> FutureReader<T> {
    #[doc(hidden)]
    pub fn new(handle: u32, vtable: &'static FutureVtable<T>) -> Self {
        Self {
            handle: AtomicU32::new(handle),
            vtable,
        }
    }

    #[doc(hidden)]
    pub fn take_handle(&self) -> u32 {
        let ret = self.opt_handle().unwrap();
        self.handle.store(u32::MAX, Relaxed);
        ret
    }

    fn handle(&self) -> u32 {
        self.opt_handle().unwrap()
    }

    fn opt_handle(&self) -> Option<u32> {
        match self.handle.load(Relaxed) {
            u32::MAX => None,
            other => Some(other),
        }
    }
}

impl<T> IntoFuture for FutureReader<T> {
    type Output = Option<T>;
    type IntoFuture = FutureRead<T>;

    /// Convert this object into a `Future` which will resolve when a value is
    /// written to the writable end of this `future` (yielding a `Some` result)
    /// or when the writable end is dropped (yielding a `None` result).
    fn into_future(self) -> Self::IntoFuture {
        FutureRead {
            op: WaitableOperation::new(self),
        }
    }
}

impl<T> Drop for FutureReader<T> {
    fn drop(&mut self) {
        let Some(handle) = self.opt_handle() else {
            return;
        };
        unsafe {
            (self.vtable.close_readable)(handle);
        }
    }
}

/// Represents a read operation which may be canceled prior to completion.
pub struct FutureRead<T: 'static> {
    op: WaitableOperation<FutureReadOp<T>>,
}

struct FutureReadOp<T>(marker::PhantomData<T>);

enum ReadComplete<T> {
    Value(T),
    Closed,
    Cancelled,
}

unsafe impl<T> WaitableOp for FutureReadOp<T>
where
    T: 'static,
{
    type Start = FutureReader<T>;
    type InProgress = (FutureReader<T>, Option<Cleanup>);
    type Result = (ReadComplete<T>, FutureReader<T>);
    type Cancel = Result<Option<T>, FutureReader<T>>;

    fn start(reader: Self::Start) -> (u32, Self::InProgress) {
        let (ptr, cleanup) = Cleanup::new(reader.vtable.layout);
        // SAFETY: `ptr` is allocated with `vtable.layout` and should be
        // safe to use here. Its lifetime for the async operation is hinged on
        // `WaitableOperation` being safe.
        let code = unsafe { (reader.vtable.start_read)(reader.handle(), ptr) };
        (code, (reader, cleanup))
    }

    fn start_cancel(state: Self::Start) -> Self::Cancel {
        Err(state)
    }

    /// The read has completed, so lift the value from the stored memory and
    /// `cleanup` naturally falls out of scope after transferring ownership of
    /// everything to the returned `value`.
    fn in_progress_complete((reader, cleanup): Self::InProgress, amt: u32) -> Self::Result {
        assert_eq!(amt, 1);
        let ptr = cleanup
            .as_ref()
            .map(|c| c.ptr.as_ptr())
            .unwrap_or(ptr::null_mut());

        // SAFETY: we're the ones managing `ptr` so we know it's safe to
        // pass here.
        let value = unsafe { (reader.vtable.lift)(ptr) };
        (ReadComplete::Value(value), reader)
    }

    /// The read didn't complete, so `_cleanup` is still uninitialized, so let
    /// it fall out of scope.
    fn in_progress_closed((reader, _cleanup): Self::InProgress) -> Self::Result {
        (ReadComplete::Closed, reader)
    }

    fn in_progress_waitable((reader, _): &Self::InProgress) -> u32 {
        reader.handle()
    }

    fn in_progress_cancel((reader, _): &Self::InProgress) -> u32 {
        // SAFETY: we're managing `reader` and all the various operational bits,
        // so this relies on `WaitableOperation` being safe.
        unsafe { (reader.vtable.cancel_read)(reader.handle()) }
    }

    /// Like `in_progress_closed` the read operation has finished but without a
    /// value, so let `_cleanup` fall out of scope to clean up its allocation.
    fn in_progress_canceled((reader, _cleanup): Self::InProgress) -> Self::Result {
        (ReadComplete::Cancelled, reader)
    }

    fn result_into_cancel((value, reader): Self::Result) -> Self::Cancel {
        match value {
            // The value was actually read, so thread that through here.
            ReadComplete::Value(value) => Ok(Some(value)),

            // The read was successfully cancelled, so thread through the
            // `reader` to possibly restart later on.
            ReadComplete::Cancelled => Err(reader),

            // The other end was closed, so this can't possibly ever complete
            // again, so thread that through.
            ReadComplete::Closed => Ok(None),
        }
    }
}

impl<T: 'static> Future for FutureRead<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pin_project()
            .poll_complete(cx)
            .map(|(result, _reader)| match result {
                ReadComplete::Value(val) => Some(val),
                ReadComplete::Cancelled | ReadComplete::Closed => None,
            })
    }
}

impl<T> FutureRead<T> {
    fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<FutureReadOp<T>>> {
        // SAFETY: we've chosen that when `Self` is pinned that it translates to
        // always pinning the inner field, so that's codified here.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
    }

    /// Cancel this read if it hasn't already completed.
    ///
    /// Return values include:
    ///
    /// * `Ok(Some(value))` - future completed before this cancellation request
    ///   was received.
    /// * `Ok(None)` - future closed before this cancellation request was
    ///   received.
    /// * `Err(reader)` - read operation was cancelled and it can be retried in
    ///   the future if desired.
    ///
    /// Note that if this method is called after the write was already cancelled
    /// then `Ok(None)` will be returned.
    ///
    /// # Panics
    ///
    /// Panics if the operation has already been completed via `Future::poll`,
    /// or if this method is called twice.
    pub fn cancel(self: Pin<&mut Self>) -> Result<Option<T>, FutureReader<T>> {
        self.pin_project().cancel()
    }
}
