use std::sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering};

use crate::{
    activate_event_send_ptr, async_support::wait_on, subscribe_event_send_ptr, EventGenerator,
};
use {
    futures::sink::Sink,
    std::{
        convert::Infallible,
        fmt,
        future::Future,
        iter,
        marker::PhantomData,
        mem::{self, ManuallyDrop, MaybeUninit},
        pin::Pin,
        task::{Context, Poll},
    },
};

fn ceiling(x: usize, y: usize) -> usize {
    (x / y) + if x % y == 0 { 0 } else { 1 }
}

pub mod results {
    pub const BLOCKED: isize = -1;
    pub const CLOSED: isize = isize::MIN;
    pub const CANCELED: isize = 0;
}

pub struct StreamWriter<T: 'static> {
    handle: StreamHandle2,
    future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    _phantom: PhantomData<T>,
}

impl<T> StreamWriter<T> {
    #[doc(hidden)]
    pub fn new(handle: *mut Stream) -> Self {
        Self {
            handle: StreamHandle2(handle),
            future: None,
            _phantom: PhantomData,
        }
    }

    pub fn cancel(&mut self) {
        todo!()
    }
}

impl<T> fmt::Debug for StreamWriter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamWriter")
            .field("handle", &self.handle.0)
            .finish()
    }
}

impl<T: Unpin> Sink<Vec<T>> for StreamWriter<T> {
    type Error = Infallible;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();

        let ready = unsafe { is_ready_to_write(me.handle.0) };

        // see also StreamReader::poll_next
        if !ready && me.future.is_none() {
            let handle = StreamHandle2(me.handle.0);
            me.future = Some(Box::pin(async move {
                let handle_local = handle;
                let subscr = unsafe { subscribe_event_send_ptr(write_ready_event(handle_local.0)) };
                subscr.reset();
                wait_on(subscr).await;
            }) as Pin<Box<dyn Future<Output = _> + Send>>);
        }

        if let Some(future) = &mut me.future {
            match future.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    me.future = None;
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, mut item: Vec<T>) -> Result<(), Self::Error> {
        let item_len = item.len();
        let me = self.get_mut();
        let stream = me.handle.0;
        let Slice { addr, size } = unsafe { start_writing(stream) };
        assert!(size >= item_len);
        let slice =
            unsafe { std::slice::from_raw_parts_mut(addr.cast::<MaybeUninit<T>>(), item_len) };
        for (a, b) in slice.iter_mut().zip(item.drain(..)) {
            a.write(b);
        }
        unsafe { finish_writing(stream, item_len as isize) };
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }
}

impl<T> Drop for StreamWriter<T> {
    fn drop(&mut self) {
        if !unsafe { is_write_closed(self.handle.0) } {
            unsafe {
                finish_writing(self.handle.0, results::CLOSED);
            }
        }
        unsafe { close_write(self.handle.0) };
    }
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T: 'static> {
    handle: StreamHandle2,
    future: Option<Pin<Box<dyn Future<Output = Option<Vec<T>>> + 'static + Send>>>,
    // event: EventSubscription,
    _phantom: PhantomData<T>,
}

impl<T> StreamReader<T> {
    /// Cancel the current pending read operation.
    ///
    /// This will panic if no such operation is pending.
    pub fn cancel(&mut self) {
        assert!(self.future.is_some());
        self.future = None;
    }
}

impl<T> fmt::Debug for StreamReader<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamReader")
            .field("handle", &self.handle.0)
            .finish()
    }
}

impl<T> StreamReader<T> {
    #[doc(hidden)]
    pub fn new(handle: *mut Stream) -> Self {
        Self {
            handle: StreamHandle2(handle),
            future: None,
            _phantom: PhantomData,
        }
    }
    #[doc(hidden)]
    pub fn into_handle(self) -> *mut Stream {
        ManuallyDrop::new(self).handle.0
    }
}

impl<T: Unpin + Send> futures::stream::Stream for StreamReader<T> {
    type Item = Vec<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let me = self.get_mut();

        if me.future.is_none() {
            let handle = StreamHandle2(me.handle.0);
            me.future = Some(Box::pin(async move {
                let mut buffer = iter::repeat_with(MaybeUninit::uninit)
                    .take(ceiling(4 * 1024, mem::size_of::<T>()))
                    .collect::<Vec<_>>();
                let stream_handle = handle;
                let result = if let Some(count) = {
                    let poll_fn = start_reading;
                    let address = super::AddressSend(buffer.as_mut_ptr() as _);
                    let count = unsafe {
                        super::await_stream_result(
                            poll_fn,
                            stream_handle,
                            address,
                            buffer.len(),
                            // &stream.event,
                        )
                        .await
                    };
                    #[allow(unused)]
                    if let Some(count) = count {
                        let value = ();
                    }
                    count
                }
                // T::read(&stream_handle, &mut buffer).await
                {
                    buffer.truncate(count);
                    Some(unsafe { mem::transmute::<Vec<MaybeUninit<T>>, Vec<T>>(buffer) })
                } else {
                    None
                };
                result
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

impl<T> Drop for StreamReader<T> {
    fn drop(&mut self) {
        unsafe { activate_event_send_ptr(write_ready_event(self.handle.0)) };
        unsafe { close_read(self.handle.0) };
    }
}

pub struct Stream {
    read_ready_event_send: *mut (),
    write_ready_event_send: *mut (),
    read_addr: AtomicPtr<()>,
    read_size: AtomicUsize,
    ready_size: AtomicIsize,
    active_instances: AtomicUsize,
}

pub unsafe extern "C" fn start_reading(stream: *mut Stream, buf: *mut (), size: usize) -> isize {
    let old_ready = unsafe { &*stream }.ready_size.load(Ordering::Acquire);
    if old_ready == results::CLOSED {
        return old_ready;
    }
    assert!(old_ready == results::BLOCKED);
    let old_size = unsafe { &mut *stream }
        .read_size
        .swap(size, Ordering::Acquire);
    assert_eq!(old_size, 0);
    let old_ptr = unsafe { &mut *stream }
        .read_addr
        .swap(buf, Ordering::Release);
    assert_eq!(old_ptr, std::ptr::null_mut());
    let write_evt = unsafe { &mut *stream }.write_ready_event_send;
    unsafe { activate_event_send_ptr(write_evt) };
    results::BLOCKED
}

pub unsafe extern "C" fn read_ready_event(stream: *const Stream) -> *mut () {
    unsafe { (&*stream).read_ready_event_send }
}

pub unsafe extern "C" fn write_ready_event(stream: *const Stream) -> *mut () {
    unsafe { (&*stream).write_ready_event_send }
}

pub unsafe extern "C" fn is_ready_to_write(stream: *const Stream) -> bool {
    !unsafe { &*stream }
        .read_addr
        .load(Ordering::Acquire)
        .is_null()
}

pub unsafe extern "C" fn is_write_closed(stream: *const Stream) -> bool {
    unsafe { &*stream }.ready_size.load(Ordering::Acquire) == results::CLOSED
}

#[repr(C)]
pub struct Slice {
    pub addr: *mut (),
    pub size: usize,
}

pub unsafe extern "C" fn start_writing(stream: *mut Stream) -> Slice {
    let size = unsafe { &*stream }.read_size.swap(0, Ordering::Acquire);
    let addr = unsafe { &*stream }
        .read_addr
        .swap(core::ptr::null_mut(), Ordering::Release);
    Slice { addr, size }
}

pub unsafe extern "C" fn read_amount(stream: *const Stream) -> isize {
    unsafe { &*stream }
        .ready_size
        .swap(results::BLOCKED, Ordering::Acquire)
}

pub unsafe extern "C" fn finish_writing(stream: *mut Stream, elements: isize) {
    let old_ready = unsafe { &*stream }
        .ready_size
        .swap(elements as isize, Ordering::Release);
    assert_eq!(old_ready, results::BLOCKED);
    unsafe { activate_event_send_ptr(read_ready_event(stream)) };
}

pub unsafe extern "C" fn close_read(stream: *mut Stream) {
    let refs = unsafe { &mut *stream }
        .active_instances
        .fetch_sub(1, Ordering::AcqRel);
    if refs == 1 {
        let obj = Box::from_raw(stream);
        drop(EventGenerator::from_handle(
            obj.read_ready_event_send as usize,
        ));
        drop(EventGenerator::from_handle(
            obj.write_ready_event_send as usize,
        ));
        drop(obj);
    }
}

pub unsafe extern "C" fn close_write(stream: *mut Stream) {
    // same for write (for now)
    close_read(stream);
}

pub extern "C" fn create_stream() -> *mut Stream {
    Box::into_raw(Box::new(Stream::new()))
}

impl Stream {
    fn new() -> Self {
        Self {
            // vtable: &STREAM_VTABLE as *const StreamVtable,
            read_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            write_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            read_addr: AtomicPtr::new(core::ptr::null_mut()),
            read_size: AtomicUsize::new(0),
            ready_size: AtomicIsize::new(results::BLOCKED),
            active_instances: AtomicUsize::new(2),
        }
    }
}

// Stream handles are Send, so wrap them
#[repr(transparent)]
pub struct StreamHandle2(pub *mut Stream);
unsafe impl Send for StreamHandle2 {}
unsafe impl Sync for StreamHandle2 {}

pub fn new_stream<T: 'static>() -> (StreamWriter<T>, StreamReader<T>) {
    let handle = create_stream();
    (StreamWriter::new(handle), StreamReader::new(handle))
}
