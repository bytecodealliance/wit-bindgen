pub use crate::module::symmetric::runtime::symmetric_stream::StreamObj as Stream;
use crate::{
    async_support::{rust_buffer::RustBuffer, wait_on},
    symmetric_stream::{Address, Buffer},
};
use {
    futures::sink::Sink,
    std::{
        alloc::Layout,
        convert::Infallible,
        fmt,
        future::Future,
        iter,
        marker::PhantomData,
        mem::MaybeUninit,
        pin::Pin,
        task::{Context, Poll},
    },
};

// waitable::{WaitableOp, WaitableOperation} looked cool, but
// as it waits for the runtime and uses Wakers I don't think the
// logic fits

#[doc(hidden)]
pub struct StreamVtable<T> {
    pub layout: Layout,
    pub lower: Option<unsafe fn(value: T, dst: *mut u8)>,
    pub lift: Option<unsafe fn(dst: *mut u8) -> T>,
    //pub dealloc_lists: Option<unsafe fn(dst: *mut u8)>,
}

// const fn ceiling(x: usize, y: usize) -> usize {
//     (x / y) + if x % y == 0 { 0 } else { 1 }
// }

pub mod results {
    pub const BLOCKED: isize = -1;
    pub const CLOSED: isize = isize::MIN;
    pub const CANCELED: isize = 0;
}

// Used within Waitable
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StreamResult {
    Complete(usize),
    Dropped,
    Cancelled,
}

pub struct StreamWrite<'a, T: 'static> {
    _phantom: PhantomData<&'a T>,
    writer: &'a mut StreamWriter<T>,
    _future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    values: RustBuffer<T>,
}

impl<T: Unpin + Send + 'static> StreamWrite<'_, T> {
    fn start_send(&mut self) -> Poll<(StreamResult, RustBuffer<T>)> {
        let mut values = RustBuffer::new(Vec::new());
        let size = if self.values.remaining() == 0 {
            // delayed flush
            // I assume EOF
            self.writer.handle.finish_writing(None);
            0
        } else {
            // send data
            let buffer = self.writer.handle.start_writing();
            let addr = buffer.get_address().take_handle() as *mut u8;
            let size = (buffer.capacity() as usize).min(self.values.remaining());
            let mut dest = addr;
            if let Some(lower) = self.writer._vtable.lower {
                for i in self.values.drain_n(size) {
                    unsafe { (lower)(i, dest) };
                    dest = unsafe { dest.byte_add(self.writer._vtable.layout.size()) };
                }
            } else {
                todo!();
            }
            buffer.set_size(size as u64);
            self.writer.handle.finish_writing(Some(buffer));
            std::mem::swap(&mut self.values, &mut values);
            size
        };
        Poll::Ready((StreamResult::Complete(size), values))
    }
}

impl<T: Unpin + Send + 'static> Future for StreamWrite<'_, T> {
    type Output = (StreamResult, RustBuffer<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        match Pin::new(&mut me.writer).poll_ready(cx) {
            Poll::Ready(_) => me.start_send(),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct StreamWriter<T: 'static> {
    handle: Stream,
    /*?*/ future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    _vtable: &'static StreamVtable<T>,
}

impl<T> StreamWriter<T> {
    #[doc(hidden)]
    pub fn new(handle: Stream, vtable: &'static StreamVtable<T>) -> Self {
        Self {
            handle,
            future: None,
            _vtable: vtable,
        }
    }

    pub fn write(&mut self, values: Vec<T>) -> StreamWrite<'_, T> {
        self.write_buf(RustBuffer::new(values)) //, self._vtable))
                                                // StreamWrite {
                                                // writer: self,
                                                // _future: None,
                                                // _phantom: PhantomData,
                                                // values,
                                                // }
    }

    pub fn write_buf(&mut self, values: RustBuffer<T>) -> StreamWrite<'_, T> {
        StreamWrite {
            writer: self,
            _future: None,
            _phantom: PhantomData,
            values,
        }
    }

    pub fn cancel(&mut self) {
        todo!()
    }
}

impl<T> fmt::Debug for StreamWriter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamWriter")
            .field("handle", &self.handle)
            .finish()
    }
}

impl<T: Unpin + Send> Sink<Vec<T>> for StreamWriter<T> {
    type Error = Infallible;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();

        let ready = me.handle.is_ready_to_write();

        // see also StreamReader::poll_next
        if !ready && me.future.is_none() {
            let handle = me.handle.clone();
            me.future = Some(Box::pin(async move {
                let handle_local = handle;
                let subscr = handle_local.write_ready_subscribe();
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

    fn start_send(self: Pin<&mut Self>, item: Vec<T>) -> Result<(), Self::Error> {
        let me = self.get_mut();
        let mut val = me.write(item);
        let poll = val.start_send();
        assert!(matches!(poll, Poll::Ready((StreamResult::Complete(_), _))));
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
        if !self.handle.is_write_closed() {
            self.handle.finish_writing(None);
        }
    }
}

/// Represents the readable end of a Component Model `stream`.
pub struct StreamReader<T: 'static> {
    handle: Stream,
    future: Option<Pin<Box<dyn Future<Output = (StreamResult, Vec<T>)> + 'static + Send>>>,
    _vtable: &'static StreamVtable<T>,
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
    pub unsafe fn new(handle: *mut u8, vtable: &'static StreamVtable<T>) -> Self {
        Self {
            handle: unsafe { Stream::from_handle(handle as usize) },
            future: None,
            _vtable: vtable,
        }
    }

    pub unsafe fn from_handle(handle: *mut u8, vtable: &'static StreamVtable<T>) -> Self {
        Self::new(handle, vtable)
    }

    /// Cancel the current pending read operation.
    ///
    /// This will panic if no such operation is pending.
    pub fn cancel(&mut self) {
        assert!(self.future.is_some());
        self.future = None;
    }

    #[doc(hidden)]
    pub fn take_handle(&self) -> usize {
        self.handle.take_handle()
    }

    #[doc(hidden)]
    // remove this as it is weirder than take_handle
    pub fn into_handle(self) -> *mut () {
        self.handle.take_handle() as *mut ()
    }

    pub fn read(&mut self, buf: Vec<T>) -> StreamRead<'_, T> {
        StreamRead { reader: self, buf }
    }
}

impl<T: Send + Unpin + 'static> StreamReader<T> {
    pub async fn next(&mut self) -> Option<T> {
        let (status, mut buf) = self.read(Vec::with_capacity(1)).await;
        match status {
            StreamResult::Complete(_) => buf.pop(),
            StreamResult::Dropped | StreamResult::Cancelled => None,
        }
    }
}

// impl<T: Unpin + Send> futures::stream::Stream for StreamReader<T> {
//     type Item = Vec<T>;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
//         let me = self.get_mut();

//         if me.future.is_none() {
//             let handle = me.handle.clone();
//             me.future = Some(Box::pin(async move {
//                 let mut buffer0 = iter::repeat_with(MaybeUninit::uninit)
//                     .take(ceiling(4 * 1024, mem::size_of::<T>()))
//                     .collect::<Vec<_>>();
//                 let address = unsafe { Address::from_handle(buffer0.as_mut_ptr() as usize) };
//                 let buffer = Buffer::new(address, buffer0.len() as u64);
//                 handle.start_reading(buffer);
//                 let subsc = handle.read_ready_subscribe();
//                 subsc.reset();
//                 wait_on(subsc).await;
//                 let buffer2 = handle.read_result();
//                 if let Some(buffer2) = buffer2 {
//                     let count = buffer2.get_size();
//                     buffer0.truncate(count as usize);
//                     // TODO: lift
//                     Some(unsafe { mem::transmute::<Vec<MaybeUninit<T>>, Vec<T>>(buffer0) })
//                 } else {
//                     None
//                 }
//             }) as Pin<Box<dyn Future<Output = _> + Send>>);
//         }

//         match me.future.as_mut().unwrap().as_mut().poll(cx) {
//             Poll::Ready(v) => {
//                 me.future = None;
//                 Poll::Ready(v)
//             }
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }

impl<T> Drop for StreamReader<T> {
    fn drop(&mut self) {
        if self.handle.handle() != 0 {
            self.handle.write_ready_activate();
        }
    }
}

pub struct StreamRead<'a, T: 'static> {
    buf: Vec<T>,
    reader: &'a mut StreamReader<T>,
}

impl<T: Unpin + Send + 'static> Future for StreamRead<'_, T> {
    type Output = (StreamResult, Vec<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me2 = self.get_mut();
        let me = &mut me2.reader;

        if me.future.is_none() {
            if me.handle.is_write_closed() {
                return Poll::Ready((StreamResult::Dropped, Vec::new()));
            }
            let mut buffer2 = Vec::new();
            std::mem::swap(&mut buffer2, &mut me2.buf);
            let handle = me.handle.clone();
            let vtable = me._vtable;
            me.future = Some(Box::pin(async move {
                let mut buffer0: Vec<MaybeUninit<u8>> = iter::repeat_with(MaybeUninit::uninit)
                    .take(vtable.layout.size() * buffer2.capacity())
                    .collect::<Vec<_>>();
                let address = unsafe { Address::from_handle(buffer0.as_mut_ptr() as usize) };
                let buffer = Buffer::new(address, buffer2.capacity() as u64);
                handle.start_reading(buffer);
                let subsc = handle.read_ready_subscribe();
                subsc.reset();
                wait_on(subsc).await;
                let buffer3 = handle.read_result();
                if let Some(buffer3) = buffer3 {
                    let count = buffer3.get_size();
                    let mut srcptr = buffer3.get_address().take_handle() as *mut u8;
                    if let Some(lift) = vtable.lift {
                        for _ in 0..count {
                            buffer2.push(unsafe { (lift)(srcptr) });
                            srcptr = unsafe { srcptr.byte_add(vtable.layout.size()) };
                        }
                    } else {
                        todo!()
                    }
                    (StreamResult::Complete(count as usize), buffer2)
                } else {
                    (StreamResult::Dropped, Vec::new())
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

/// deprecate this, replace with stream_new
pub fn new_stream<T: 'static>(
    vtable: &'static StreamVtable<T>,
) -> (StreamWriter<T>, StreamReader<T>) {
    let handle = Stream::new();
    let handle2 = handle.clone();
    (StreamWriter::new(handle, vtable), unsafe {
        StreamReader::new(handle2.take_handle() as *mut u8, vtable)
    })
}

pub fn stream_new<T: 'static>(
    vtable: &'static StreamVtable<T>,
) -> (StreamWriter<T>, StreamReader<T>) {
    new_stream(vtable)
}
