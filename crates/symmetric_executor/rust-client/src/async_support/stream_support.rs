pub use crate::module::symmetric::runtime::symmetric_stream::StreamObj as Stream;
use crate::{
    async_support::{
        rust_buffer::RustBuffer,
        wait_on,
        waitable::{WaitableOp, WaitableOperation},
    },
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
        marker::{self, PhantomData},
        mem::{self, MaybeUninit},
        pin::Pin,
        task::{Context, Poll},
    },
};

#[doc(hidden)]
pub struct StreamVtable<T> {
    pub layout: Layout,
    pub lower: Option<unsafe fn(value: T, dst: *mut u8)>,
    pub lift: Option<unsafe fn(dst: *mut u8) -> T>,
    //pub dealloc_lists: Option<unsafe fn(dst: *mut u8)>,
}

const fn ceiling(x: usize, y: usize) -> usize {
    (x / y) + if x % y == 0 { 0 } else { 1 }
}

pub mod results {
    pub const BLOCKED: isize = -1;
    pub const CLOSED: isize = isize::MIN;
    pub const CANCELED: isize = 0;
}

// Used within Waitable
pub mod new_state {
    pub const DROPPED: u32 = 1;
    pub const WAITING_FOR_READY: u32 = 2;
    pub const WAITING_FOR_FINISH: u32 = 3;
    pub const FINISHED: u32 = 0;
    pub const UNKNOWN: u32 = 4;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StreamResult {
    Complete(usize),
    Dropped,
    Cancelled,
}

pub struct StreamWrite<'a, T: 'static> {
    op: WaitableOperation<StreamWriteOp<'a, T>>,
    // _phantom: PhantomData<&'a T>,
    // writer: &'a mut StreamWriter<T>,
    // _future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    // values: Vec<T>,
}

struct WriteInProgress<'a, T: 'static> {
    writer: &'a mut StreamWriter<T>,
    buf: RustBuffer<T>,
    event: Option<crate::EventSubscription>,
    amount: usize,
}

unsafe impl<'a, T> WaitableOp for StreamWriteOp<'a, T>
where
    T: 'static,
{
    type Start = (&'a mut StreamWriter<T>, RustBuffer<T>);
    type InProgress = WriteInProgress<'a, T>;
    type Result = (StreamResult, RustBuffer<T>);
    type Cancel = (StreamResult, RustBuffer<T>);
    type Handle = crate::EventSubscription;

    fn start((writer, buf): Self::Start) -> (u32, Self::InProgress) {
        (
            new_state::UNKNOWN,
            WriteInProgress {
                writer,
                buf,
                event: None,
                amount: 0,
            },
        )
    }
    fn in_progress_update(
        WriteInProgress {
            writer,
            mut buf,
            event,
            mut amount,
        }: Self::InProgress,
        code: u32,
    ) -> Result<Self::Result, Self::InProgress> {
        loop {
            if writer.done {
                return Ok((
                    if amount == 0 {
                        StreamResult::Dropped
                    } else {
                        StreamResult::Complete(amount)
                    },
                    buf,
                ));
            }

            // was poll_ready
            let ready = writer.handle.is_ready_to_write();

            if !ready {
                let subscr = writer.handle.write_ready_subscribe();
                subscr.reset();
                break Err(WriteInProgress {
                    writer,
                    buf,
                    event: Some(subscr),
                    amount,
                });
                //(new_state::WAITING_FOR_READY, (writer, buf, Some(subscr)));
            } else {
                // was start_send
                let buffer = writer.handle.start_writing();
                let addr = buffer.get_address().take_handle() as *mut u8;
                let size = buffer.capacity() as usize;
                buf.take_n(size, |v| todo!());
                writer.handle.finish_writing(Some(buffer));
                amount += size;
                if buf.remaining() == 0 {
                    break Ok((StreamResult::Complete(amount), buf));
                    //(new_state::FINISHED, (writer, buf, None));
                }
            }
        }
    }
    fn start_cancelled((writer, buf): Self::Start) -> Self::Cancel {
        todo!()
        //        WriteInProgress{StreamResult::Cancelled, writer, buf, amount: 0}
    }
    fn in_progress_waitable(_progr: &Self::InProgress) -> Self::Handle {
        if let Some(evt) = _progr.event.as_ref() {
            evt.dup()
        //            symmetric_executor::register()
        } else {
            todo!()
        }
        // writer.handle.handle() as crate::async_support::waitable::Handle
    }
    fn in_progress_cancel(_progr: &Self::InProgress) -> u32 {
        todo!()
    }
    fn result_into_cancel(result: Self::Result) -> Self::Cancel {
        result
    }
}

struct StreamWriteOp<'a, T: 'static>(marker::PhantomData<(&'a mut StreamWriter<T>, T)>);

impl<T: Unpin + Send + 'static> Future for StreamWrite<'_, T> {
    type Output = (StreamResult, RustBuffer<T>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pin_project().poll_complete(cx)
        // todo!()
        // let me = self.get_mut();
        // match Pin::new(&mut me.writer).poll_ready(cx) {
        //     Poll::Ready(_) => {
        //         let values: Vec<_> = me.values.drain(..).collect();
        //         if values.is_empty() {
        //             // delayed flush
        //             Poll::Ready((
        //                 StreamResult::Complete(1),
        //                 RustBuffer::new(Vec::new(), me.writer._vtable),
        //             ))
        //         } else {
        //             Pin::new(&mut me.writer).start_send(values).unwrap();
        //             match Pin::new(&mut me.writer).poll_ready(cx) {
        //                 Poll::Ready(_) => Poll::Ready((
        //                     StreamResult::Complete(1),
        //                     RustBuffer::new(Vec::new(), me.writer._vtable),
        //                 )),
        //                 Poll::Pending => Poll::Pending,
        //             }
        //         }
        //     }
        //     Poll::Pending => Poll::Pending,
        // }
    }
}

impl<'a, T: 'static> StreamWrite<'a, T> {
    fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<StreamWriteOp<'a, T>>> {
        // SAFETY: we've chosen that when `Self` is pinned that it translates to
        // always pinning the inner field, so that's codified here.
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
    }

    pub fn cancel(self: Pin<&mut Self>) -> (StreamResult, RustBuffer<T>) {
        self.pin_project().cancel()
    }
}

pub struct StreamWriter<T: 'static> {
    handle: Stream,
    /*?*/ future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    _vtable: &'static StreamVtable<T>,
    done: bool,
}

impl<T> StreamWriter<T> {
    #[doc(hidden)]
    pub fn new(handle: Stream, vtable: &'static StreamVtable<T>) -> Self {
        Self {
            handle,
            future: None,
            _vtable: vtable,
            done: false,
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
            op: WaitableOperation::new((self, values)),
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

impl<T: Unpin> Sink<Vec<T>> for StreamWriter<T> {
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

    fn start_send(self: Pin<&mut Self>, mut item: Vec<T>) -> Result<(), Self::Error> {
        let item_len = item.len();
        let me = self.get_mut();
        let stream = &me.handle;
        let buffer = stream.start_writing();
        let addr = buffer.get_address().take_handle() as *mut u8;
        let size = buffer.capacity() as usize;
        assert!(size >= item_len);
        let slice =
            unsafe { std::slice::from_raw_parts_mut(addr.cast::<MaybeUninit<T>>(), item_len) };
        for (a, b) in slice.iter_mut().zip(item.drain(..)) {
            // TODO: lower
            a.write(b);
        }
        buffer.set_size(item_len as u64);
        stream.finish_writing(Some(buffer));
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
    future: Option<Pin<Box<dyn Future<Output = Option<Vec<T>>> + 'static + Send>>>,
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
        StreamRead {
            // marker: PhantomData,
            reader: self,
            buf,
        }
    }
}

impl<T: Unpin + Send> futures::stream::Stream for StreamReader<T> {
    type Item = Vec<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let me = self.get_mut();

        if me.future.is_none() {
            let handle = me.handle.clone();
            me.future = Some(Box::pin(async move {
                let mut buffer0 = iter::repeat_with(MaybeUninit::uninit)
                    .take(ceiling(4 * 1024, mem::size_of::<T>()))
                    .collect::<Vec<_>>();
                let address = unsafe { Address::from_handle(buffer0.as_mut_ptr() as usize) };
                let buffer = Buffer::new(address, buffer0.len() as u64);
                handle.start_reading(buffer);
                let subsc = handle.read_ready_subscribe();
                subsc.reset();
                wait_on(subsc).await;
                let buffer2 = handle.read_result();
                if let Some(buffer2) = buffer2 {
                    let count = buffer2.get_size();
                    buffer0.truncate(count as usize);
                    // TODO: lift
                    Some(unsafe { mem::transmute::<Vec<MaybeUninit<T>>, Vec<T>>(buffer0) })
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

impl<T> Drop for StreamReader<T> {
    fn drop(&mut self) {
        if self.handle.handle() != 0 {
            self.handle.write_ready_activate();
        }
    }
}

pub struct StreamRead<'a, T: 'static> {
    // marker: PhantomData<(&'a mut StreamReader<T>, T)>,
    buf: Vec<T>,
    reader: &'a mut StreamReader<T>,
}

impl<T: 'static> Future for StreamRead<'_, T> {
    type Output = (StreamResult, Vec<T>);

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: Check whether WaitableOperation helps here
        //self.pin_project().poll_complete(cx)

        todo!()

        // let me2 = self.get_mut();
        // let me = &mut me2.reader;

        // if me.future.is_none() {
        //     let mut buffer2 = Vec::new();
        //     std::mem::swap(&mut buffer2, &mut me2.buf);
        //     let handle = me.handle.clone();
        //     me.future = Some(Box::pin(async move {
        //         let mut buffer0 = iter::repeat_with(MaybeUninit::uninit)
        //             .take(ceiling(4 * 1024, mem::size_of::<T>()))
        //             .collect::<Vec<_>>();
        //         let address = unsafe { Address::from_handle(buffer0.as_mut_ptr() as usize) };
        //         let buffer = Buffer::new(address, buffer0.capacity() as u64);
        //         handle.start_reading(buffer);
        //         let subsc = handle.read_ready_subscribe();
        //         subsc.reset();
        //         wait_on(subsc).await;
        //         let buffer2 = handle.read_result();
        //         if let Some(buffer2) = buffer2 {
        //             let count = buffer2.get_size();
        //             buffer0.truncate(count as usize);
        //             Some(unsafe { mem::transmute::<Vec<MaybeUninit<T>>, Vec<T>>(buffer0) })
        //         } else {
        //             None
        //         }
        //     }) as Pin<Box<dyn Future<Output = _> + Send>>);
        // }

        // match me.future.as_mut().unwrap().as_mut().poll(cx) {
        //     Poll::Ready(v) => {
        //         me.future = None;
        //         Poll::Ready(v)
        //     }
        //     Poll::Pending => Poll::Pending,
        // }
    }
}

// impl<'a, T> StreamRead<'a, T> {
//     fn pin_project(self: Pin<&mut Self>) -> Pin<&mut WaitableOperation<StreamReadOp<'a, T>>> {
//         // SAFETY: we've chosen that when `Self` is pinned that it translates to
//         // always pinning the inner field, so that's codified here.
//         unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().op) }
//     }
// }

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
