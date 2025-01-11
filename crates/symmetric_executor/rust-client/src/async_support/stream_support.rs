//use std::sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering};

pub use crate::module::symmetric::runtime::symmetric_stream::StreamObj as Stream;
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
    pub fn new(handle: Stream) -> Self {
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

        let ready = me.handle.0.is_ready_to_write();

        // see also StreamReader::poll_next
        if !ready && me.future.is_none() {
            let handle = StreamHandle2(me.handle.0);
            me.future = Some(Box::pin(async move {
                let handle_local = handle;
                let subscr = handle_local.0.write_ready_event();
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
        let Slice { addr, size } = stream.start_writing();
        assert!(size >= item_len);
        let slice =
            unsafe { std::slice::from_raw_parts_mut(addr.cast::<MaybeUninit<T>>(), item_len) };
        for (a, b) in slice.iter_mut().zip(item.drain(..)) {
            a.write(b);
        }
        stream.finish_writing(item_len as isize);
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
        if !self.handle.0.is_write_closed() {
            self.handle.0.finish_writing(results::CLOSED);
        }
        self.handle.0.close_write();
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
    pub fn new(handle: Stream) -> Self {
        Self {
            handle: StreamHandle2(handle),
            future: None,
            _phantom: PhantomData,
        }
    }
    #[doc(hidden)]
    pub fn into_handle(self) -> *mut () {
        self.handle.0.take_handle() as *mut ()
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
        self.handle.0.write_ready_event().activate();
        Stream::close_read(self.handle.0);
        //        unsafe { activate_event_send_ptr(write_ready_event(self.handle.0)) };
        // unsafe { close_read(self.handle.0) };
    }
}

// Stream handles are Send, so wrap them
#[repr(transparent)]
pub struct StreamHandle2(Stream);
unsafe impl Send for StreamHandle2 {}
unsafe impl Sync for StreamHandle2 {}

pub fn new_stream<T: 'static>() -> (StreamWriter<T>, StreamReader<T>) {
    let handle = Stream::new();
    (StreamWriter::new(handle), StreamReader::new(handle))
}
