use core::ptr::{self, NonNull};
use std::{
    alloc::{self, Layout},
    future::{Future, IntoFuture},
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};

use futures::FutureExt;

use crate::symmetric_stream::{Address, Buffer};

use super::{wait_on, Stream};

#[doc(hidden)]
pub struct FutureVtable<T> {
    pub layout: Layout,
    pub lower: unsafe fn(value: T, dst: *mut u8),
    pub lift: unsafe fn(dst: *mut u8) -> T,
}

// stolen from guest-rust/rt/src/lib.rs
pub struct Cleanup {
    ptr: NonNull<u8>,
    layout: Layout,
}

// Usage of the returned pointer is always unsafe and must abide by these
// conventions, but this structure itself has no inherent reason to not be
// send/sync.
unsafe impl Send for Cleanup {}
unsafe impl Sync for Cleanup {}

impl Cleanup {
    pub fn new(layout: Layout) -> (*mut u8, Option<Cleanup>) {
        if layout.size() == 0 {
            return (ptr::null_mut(), None);
        }
        let ptr = unsafe { alloc::alloc(layout) };
        let ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => alloc::handle_alloc_error(layout),
        };
        (ptr.as_ptr(), Some(Cleanup { ptr, layout }))
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

pub struct FutureWriter<T: 'static> {
    handle: Stream,
    vtable: &'static FutureVtable<T>,
}

impl<T> FutureWriter<T> {
    pub fn new(handle: Stream, vtable: &'static FutureVtable<T>) -> Self {
        Self { handle, vtable }
    }

    pub fn write(self, data: T) -> CancelableWrite<T> {
        CancelableWrite {
            writer: self,
            future: None,
            data: Some(data),
        }
    }
}

/// Represents a write operation which may be canceled prior to completion.
pub struct CancelableWrite<T: 'static> {
    writer: FutureWriter<T>,
    future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    data: Option<T>,
}

impl<T: Unpin + Send> Future for CancelableWrite<T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let me = self.get_mut();

        if me.future.is_none() {
            let handle = me.writer.handle.clone();
            let data = me.data.take().unwrap();
            let lower = me.writer.vtable.lower;
            me.future = Some(Box::pin(async move {
                if !handle.is_ready_to_write() {
                    let subsc = handle.write_ready_subscribe();
                    wait_on(subsc).await;
                }
                let buffer = handle.start_writing();
                let addr = buffer.get_address().take_handle() as *mut MaybeUninit<T> as *mut u8;
                unsafe { (lower)(data, addr) };
                buffer.set_size(1);
                handle.finish_writing(Some(buffer));
            }) as Pin<Box<dyn Future<Output = _> + Send>>);
        }
        me.future.as_mut().unwrap().poll_unpin(cx)
    }
}

/// Represents a read operation which may be canceled prior to completion.
pub struct CancelableRead<T: 'static> {
    reader: FutureReader<T>,
    future: Option<Pin<Box<dyn Future<Output = Option<T>> + 'static + Send>>>,
}

pub struct FutureReader<T: 'static> {
    handle: Stream,
    vtable: &'static FutureVtable<T>,
}

impl<T> FutureReader<T> {
    pub fn new(handle: Stream, vtable: &'static FutureVtable<T>) -> Self {
        Self { handle, vtable }
    }

    pub fn read(self) -> CancelableRead<T> {
        CancelableRead {
            reader: self,
            future: None,
        }
    }

    pub unsafe fn from_handle(handle: *mut u8, vtable: &'static FutureVtable<T>) -> Self {
        Self::new(unsafe { Stream::from_handle(handle as usize) }, vtable)
    }

    pub fn take_handle(&self) -> *mut () {
        self.handle.take_handle() as *mut ()
    }
}

impl<T: Unpin + Sized + Send> Future for CancelableRead<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        let me = self.get_mut();

        if me.future.is_none() {
            let handle = me.reader.handle.clone();
            let vtable = me.reader.vtable;
            me.future = Some(Box::pin(async move {
                // sadly there is no easy way to embed this in the future as the size is not accessible at compile time
                let (buffer0, cleanup) = Cleanup::new(vtable.layout);
                let address = unsafe { Address::from_handle(buffer0 as usize) };
                let buffer = Buffer::new(address, 1);
                handle.start_reading(buffer);
                let subsc = handle.read_ready_subscribe();
                subsc.reset();
                wait_on(subsc).await;
                let buffer2 = handle.read_result();
                if let Some(buffer2) = buffer2 {
                    let count = buffer2.get_size();
                    if count > 0 {
                        Some(unsafe {
                            (vtable.lift)(buffer2.get_address().take_handle() as *mut u8)
                        })
                    } else {
                        // make sure it lives long enough
                        drop(cleanup);
                        None
                    }
                } else {
                    None
                }
            }) as Pin<Box<dyn Future<Output = _> + Send>>);
        }

        match me.future.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Ready(v) => {
                me.future = None;
                Poll::Ready(v)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> CancelableRead<T> {
    pub fn cancel(mut self) -> FutureReader<T> {
        self.cancel_mut()
    }

    fn cancel_mut(&mut self) -> FutureReader<T> {
        todo!()
    }
}

impl<T: Send + Unpin + Sized> IntoFuture for FutureReader<T> {
    type Output = Option<T>;
    type IntoFuture = CancelableRead<T>;

    /// Convert this object into a `Future` which will resolve when a value is
    /// written to the writable end of this `future` (yielding a `Some` result)
    /// or when the writable end is dropped (yielding a `None` result).
    fn into_future(self) -> Self::IntoFuture {
        self.read()
    }
}

pub fn new_future<T: 'static>(
    vtable: &'static FutureVtable<T>,
) -> (FutureWriter<T>, FutureReader<T>) {
    let handle = Stream::new();
    let handle2 = handle.clone();
    (
        FutureWriter::new(handle, vtable),
        FutureReader::new(handle2, vtable),
    )
}
