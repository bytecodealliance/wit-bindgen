use crate::async_support::waitable::{WaitableOp, WaitableOperation};
use crate::async_support::AbiBuffer;
use {
    crate::Cleanup,
    std::{
        alloc::Layout,
        fmt,
        future::Future,
        marker,
        pin::Pin,
        ptr,
        sync::atomic::{AtomicU32, Ordering::Relaxed},
        task::{Context, Poll},
        vec::Vec,
    },
};

#[doc(hidden)]
pub struct StreamVtable<T> {
    pub layout: Layout,
    pub lower: Option<unsafe fn(value: T, dst: *mut u8)>,
    pub dealloc_lists: Option<unsafe fn(dst: *mut u8)>,
    pub lift: Option<unsafe fn(dst: *mut u8) -> T>,
    pub start_write: unsafe extern "C" fn(stream: u32, val: *const u8, amt: usize) -> u32,
    pub start_read: unsafe extern "C" fn(stream: u32, val: *mut u8, amt: usize) -> u32,
    pub cancel_write: unsafe extern "C" fn(stream: u32) -> u32,
    pub cancel_read: unsafe extern "C" fn(stream: u32) -> u32,
    pub close_writable: unsafe extern "C" fn(stream: u32),
    pub close_readable: unsafe extern "C" fn(stream: u32),
    pub new: unsafe extern "C" fn() -> u64,
}

/// Helper function to create a new read/write pair for a component model
/// stream.
pub unsafe fn stream_new<T>(
    vtable: &'static StreamVtable<T>,
) -> (StreamWriter<T>, StreamReader<T>) {
    unsafe {
        let handles = (vtable.new)();
        (
            StreamWriter::new(handles as u32, vtable),
            StreamReader::new((handles >> 32) as u32, vtable),
        )
    }
}

/// Represents the writable end of a Component Model `stream`.
pub struct StreamWriter<T: 'static> {
    handle: u32,
    vtable: &'static StreamVtable<T>,
}

impl<T> StreamWriter<T> {
    #[doc(hidden)]
    pub unsafe fn new(handle: u32, vtable: &'static StreamVtable<T>) -> Self {
        Self { handle, vtable }
    }

    /// Initiate a write of the `values` provided into this stream.
    ///
    /// This method will initiate a single write of the `values` provided. Upon
    /// completion the values will be yielded back as an [`AbiBuffer<T>`] which
    /// manages intermediate state. That can be used to resume after a partial
    /// write or re-acquire the underlying storage.
    pub fn write(&mut self, values: Vec<T>) -> StreamWrite<'_, T> {
        self.write_buf(AbiBuffer::new(values, self.vtable))
    }

    /// Same as [`StreamWriter::write`], except this takes [`AbiBuffer<T>`]
    /// instead of `Vec<T>`.
    pub fn write_buf(&mut self, values: AbiBuffer<T>) -> StreamWrite<'_, T> {
        StreamWrite {
            op: WaitableOperation::new((self, values)),
        }
    }

    /// Writes all of the `values` provided into this stream.
    ///
    /// This is a higher-level method than [`StreamWriter::write`] and does not
    /// expose cancellation for example. This will successively attempt to write
    /// all of `values` provided into this stream. Upon completion the same
    /// vector will be returned and any remaining elements in the vector were
    /// not sent because the stream was closed.
    pub async fn write_all(&mut self, values: Vec<T>) -> Vec<T> {
        let (mut status, mut buf) = self.write(values).await;
        while let StreamResult::Complete(_) = status {
            if buf.remaining() == 0 {
                break;
            }
            (status, buf) = self.write_buf(buf).await;
        }
        assert!(buf.remaining() == 0 || matches!(status, StreamResult::Closed));
        buf.into_vec()
    }

    /// Writes the singular `value` provided
    ///
    /// This is a higher-level method than [`StreamWriter::write`] and does not
    /// expose cancellation for example. This will attempt to send `value` on
    /// this stream.
    ///
    /// If the other end hangs up then the value is returned back as
    /// `Some(value)`, otherwise `None` is returned indicating the value was
    /// sent.
    pub async fn write_one(&mut self, value: T) -> Option<T> {
        self.write_all(std::vec![value]).await.pop()
    }
}

impl<T> fmt::Debug for StreamWriter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamWriter")
            .field("handle", &self.handle)
            .finish()
    }
}

impl<T> Drop for StreamWriter<T> {
    fn drop(&mut self) {
        unsafe {
            (self.vtable.close_writable)(self.handle);
        }
    }
}

/// Represents a write operation which may be canceled prior to completion.
pub struct StreamWrite<'a, T: 'static> {
    op: WaitableOperation<StreamWriteOp<'a, T>>,
}

struct StreamWriteOp<'a, T: 'static>(marker::PhantomData<(&'a mut StreamWriter<T>, T)>);

/// Result of a [`StreamWriter::write`] or [`StreamReader::read`] operation,
/// yielded by the [`StreamWrite`] or [`StreamRead`] futures.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StreamResult {
    /// The provided number of values were successfully transferred.
    ///
    /// For writes this is how many items were written, and for reads this is
    /// how many items were read.
    Complete(usize),
    /// No values were written, the other end has closed its handle.
    Closed,
    /// No values were written, the operation was cancelled.
    Cancelled,
}

unsafe impl<'a, T> WaitableOp for StreamWriteOp<'a, T>
where
    T: 'static,
{
    type Start = (&'a mut StreamWriter<T>, AbiBuffer<T>);
    type InProgress = (&'a mut StreamWriter<T>, AbiBuffer<T>);
    type Result = (StreamResult, AbiBuffer<T>);
    type Cancel = (StreamResult, AbiBuffer<T>);

    fn start((writer, buf): Self::Start) -> (u32, Self::InProgress) {
        let (ptr, len) = buf.abi_ptr_and_len();
        // SAFETY: sure hope this is safe, everything in this module and
        // `AbiBuffer` is trying to make this safe.
        let code = unsafe { (writer.vtable.start_write)(writer.handle, ptr, len) };
        (code, (writer, buf))
    }

    fn start_cancel((_writer, buf): Self::Start) -> Self::Cancel {
        (StreamResult::Cancelled, buf)
    }

    fn in_progress_complete((_writer, mut buf): Self::InProgress, amt: u32) -> Self::Result {
        let amt = amt.try_into().unwrap();
        buf.advance(amt);
        (StreamResult::Complete(amt), buf)
    }

    fn in_progress_closed((_writer, buf): Self::InProgress) -> Self::Result {
        (StreamResult::Closed, buf)
    }

    fn in_progress_waitable((writer, _): &Self::InProgress) -> u32 {
        writer.handle
    }

    fn in_progress_cancel((writer, _): &Self::InProgress) -> u32 {
        // SAFETY: we're managing `writer` and all the various operational bits,
        // so this relies on `WaitableOperation` being safe.
        unsafe { (writer.vtable.cancel_write)(writer.handle) }
    }

    fn in_progress_canceled((_writer, buf): Self::InProgress) -> Self::Result {
        (StreamResult::Cancelled, buf)
    }

    fn result_into_cancel(result: Self::Result) -> Self::Cancel {
        result
    }
}

impl<T: 'static> Future for StreamWrite<'_, T> {
    type Output = (StreamResult, AbiBuffer<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pin_project().poll_complete(cx)
    }
}

impl<'a, T: 'static> StreamWrite<'a, T> {
    fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<StreamWriteOp<'a, T>>> {
        // SAFETY: we've chosen that when `Self` is pinned that it translates to
        // always pinning the inner field, so that's codified here.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
    }

    /// Cancel this write if it hasn't already completed.
    ///
    /// This method can be used to cancel a write-in-progress and re-acquire
    /// values being sent. Note that the result here may still indicate that
    /// some values were written if the race to cancel the write was lost.
    ///
    /// # Panics
    ///
    /// Panics if the operation has already been completed via `Future::poll`,
    /// or if this method is called twice.
    pub fn cancel(self: Pin<&mut Self>) -> (StreamResult, AbiBuffer<T>) {
        self.pin_project().cancel()
    }
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T: 'static> {
    handle: AtomicU32,
    vtable: &'static StreamVtable<T>,
}

impl<T> fmt::Debug for StreamReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamReader")
            .field("handle", &self.handle)
            .finish()
    }
}

impl<T> StreamReader<T> {
    #[doc(hidden)]
    pub fn new(handle: u32, vtable: &'static StreamVtable<T>) -> Self {
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

    /// Starts a new read operation on this stream into `buf`.
    ///
    /// This method will read values into the spare capacity of the `buf`
    /// provided. If `buf` has no spare capacity then this will be equivalent
    /// to a zero-length read.
    ///
    /// Upon completion the `buf` will be yielded back to the caller via the
    /// completion of the [`StreamRead`] future.
    pub fn read(&mut self, buf: Vec<T>) -> StreamRead<'_, T> {
        StreamRead {
            op: WaitableOperation::new((self, buf)),
        }
    }

    /// Reads a single item from this stream.
    ///
    /// This is a higher-level method than [`StreamReader::read`] in that it
    /// reads only a single item and does not expose control over cancellation.
    pub async fn next(&mut self) -> Option<T> {
        // TODO: should amortize this allocation and avoid doing it every time.
        // Or somehow perhaps make this more optimal.
        let (_result, mut buf) = self.read(Vec::with_capacity(1)).await;
        buf.pop()
    }

    /// Reads all items from this stream and returns the list.
    ///
    /// This method will read all remaining items from this stream into a list
    /// and await the stream to be closed.
    pub async fn collect(mut self) -> Vec<T> {
        let mut ret = Vec::new();
        loop {
            if ret.len() == ret.capacity() {
                ret.reserve(1);
            }
            let (status, buf) = self.read(ret).await;
            ret = buf;
            match status {
                StreamResult::Complete(_) => {}
                StreamResult::Closed => break,
                StreamResult::Cancelled => unreachable!(),
            }
        }
        ret
    }
}

impl<T> Drop for StreamReader<T> {
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
pub struct StreamRead<'a, T: 'static> {
    op: WaitableOperation<StreamReadOp<'a, T>>,
}

struct StreamReadOp<'a, T: 'static>(marker::PhantomData<(&'a mut StreamReader<T>, T)>);

unsafe impl<'a, T> WaitableOp for StreamReadOp<'a, T>
where
    T: 'static,
{
    type Start = (&'a mut StreamReader<T>, Vec<T>);
    type InProgress = (&'a mut StreamReader<T>, Vec<T>, Option<Cleanup>);
    type Result = (StreamResult, Vec<T>);
    type Cancel = (StreamResult, Vec<T>);

    fn start((reader, mut buf): Self::Start) -> (u32, Self::InProgress) {
        let cap = buf.spare_capacity_mut();
        let ptr;
        let cleanup;
        // If `T` requires a lifting operation, then allocate a slab of memory
        // which will store the canonical ABI read. Otherwise we can use the
        // raw capacity in `buf` itself.
        if reader.vtable.lift.is_some() {
            let layout = Layout::from_size_align(
                reader.vtable.layout.size() * cap.len(),
                reader.vtable.layout.align(),
            )
            .unwrap();
            (ptr, cleanup) = Cleanup::new(layout);
        } else {
            ptr = cap.as_mut_ptr().cast();
            cleanup = None;
        }
        // SAFETY: `ptr` is either in `buf` or in `cleanup`, both of which will
        // persist with this async operation itself.
        let code = unsafe { (reader.vtable.start_read)(reader.handle(), ptr, cap.len()) };
        (code, (reader, buf, cleanup))
    }

    fn start_cancel((_, buf): Self::Start) -> Self::Cancel {
        std::dbg!("start_cancel");
        (StreamResult::Cancelled, buf)
    }

    fn in_progress_complete(
        (reader, mut buf, cleanup): Self::InProgress,
        amt: u32,
    ) -> Self::Result {
        let amt = usize::try_from(amt).unwrap();
        let cur_len = buf.len();
        assert!(amt <= buf.capacity() - cur_len);

        match reader.vtable.lift {
            // With a `lift` operation this now requires reading `amt` items
            // from `cleanup` and pushing them into `buf`.
            Some(lift) => {
                let mut ptr = cleanup
                    .as_ref()
                    .map(|c| c.ptr.as_ptr())
                    .unwrap_or(ptr::null_mut());
                for _ in 0..amt {
                    unsafe {
                        buf.push(lift(ptr));
                        ptr = ptr.add(reader.vtable.layout.size());
                    }
                }
            }

            // If no `lift` was necessary, then the results of this operation
            // were read directly into `buf`, so just update its length now that
            // values have been initialized.
            None => unsafe { buf.set_len(cur_len + amt) },
        }

        // Intentionally dispose of `cleanup` here as, if it was used, all
        // allocations have been read from it and appended to `buf`.
        drop(cleanup);
        (StreamResult::Complete(amt), buf)
    }

    /// Like `in_progress_canceled` below, discard the temporary cleanup
    /// allocation, if any.
    fn in_progress_closed((_reader, buf, _cleanup): Self::InProgress) -> Self::Result {
        (StreamResult::Closed, buf)
    }

    fn in_progress_waitable((reader, ..): &Self::InProgress) -> u32 {
        reader.handle()
    }

    fn in_progress_cancel((reader, ..): &Self::InProgress) -> u32 {
        // SAFETY: we're managing `reader` and all the various operational bits,
        // so this relies on `WaitableOperation` being safe.
        unsafe { (reader.vtable.cancel_read)(reader.handle()) }
    }

    /// When an in-progress read is successfully cancel then the allocation
    /// that was being read into, if any, is just discarded.
    ///
    /// TODO: should maybe thread this around like `AbiBuffer` to cache the
    /// read allocation?
    fn in_progress_canceled((_reader, buf, _cleanup): Self::InProgress) -> Self::Result {
        (StreamResult::Cancelled, buf)
    }

    fn result_into_cancel(result: Self::Result) -> Self::Cancel {
        result
    }
}

impl<T: 'static> Future for StreamRead<'_, T> {
    type Output = (StreamResult, Vec<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pin_project().poll_complete(cx)
    }
}

impl<'a, T> StreamRead<'a, T> {
    fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<StreamReadOp<'a, T>>> {
        // SAFETY: we've chosen that when `Self` is pinned that it translates to
        // always pinning the inner field, so that's codified here.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
    }

    /// Cancel this read if it hasn't already completed.
    ///
    /// This method will initiate a cancellation operation for this active
    /// read. This may race with the actual read itself and so this may actually
    /// complete with some results.
    ///
    /// The final result of cancellation is returned, along with the original
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if the operation has already been completed via `Future::poll`,
    /// or if this method is called twice.
    pub fn cancel(self: Pin<&mut Self>) -> (StreamResult, Vec<T>) {
        todo!()
    }
}
