use std::{
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use super::Stream;

//use super::Future;

pub struct FutureWriter<T: 'static> {
    handle: Stream,
    future: Option<Pin<Box<dyn Future<Output = ()> + 'static + Send>>>,
    _phantom: PhantomData<T>,
}

impl<T> FutureWriter<T> {
    pub fn new(handle: Stream) -> Self {
        Self {
            handle,
            future: None,
            _phantom: PhantomData,
        }
    }

    pub fn write(self, _v: T) {
        todo!()
    }
}

/// Represents a read operation which may be canceled prior to completion.
pub struct CancelableRead<T: 'static> {
    reader: Option<FutureReader<T>>,
    future: Pin<Box<dyn Future<Output = Option<T>> + 'static + Send>>,
}

pub struct FutureReader<T: 'static> {
    handle: Stream,
    // future: Option<Pin<Box<dyn Future<Output = Option<Vec<T>>> + 'static + Send>>>,
    _phantom: PhantomData<T>,
}

impl<T> FutureReader<T> {
    pub fn new(handle: Stream) -> Self {
        Self {
            handle,
            // future: None,
            _phantom: PhantomData,
        }
    }

    pub fn read(self) -> CancelableRead<T> {
        CancelableRead{ reader: Some(self), future: todo!() }
    }

    pub fn take_handle(&self) -> *mut () {
        self.handle.take_handle() as *mut ()
    }
}

impl<T> Future for CancelableRead<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        todo!()
    }
}

impl<T> IntoFuture for FutureReader<T> {
    type Output = Option<T>;
    type IntoFuture = CancelableRead<T>;

    /// Convert this object into a `Future` which will resolve when a value is
    /// written to the writable end of this `future` (yielding a `Some` result)
    /// or when the writable end is dropped (yielding a `None` result).
    fn into_future(self) -> Self::IntoFuture {
        self.read()
    }
}

pub fn new_future<T: 'static>() -> (FutureWriter<T>, FutureReader<T>) {
    let handle = Stream::new();
    let handle2 = handle.clone();
    (FutureWriter::new(handle), FutureReader::new(handle2))
}
