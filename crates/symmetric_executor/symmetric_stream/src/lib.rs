use std::{mem::transmute, sync::atomic::{AtomicUsize, Ordering}};

use stream_impl::exports::symmetric::runtime::symmetric_stream::{self, Address, GuestAddress, GuestBuffer, GuestStreamObj};

mod stream_impl;

struct Guest;

stream_impl::export!(Guest with_types_in stream_impl);

struct Dummy;

impl GuestAddress for Dummy {}

struct Buffer {
    addr: *mut (),
    capacity: usize,
    size: AtomicUsize,
}

impl GuestBuffer for Buffer {
    fn new(addr: symmetric_stream::Address,capacity: u64,) -> Self {
        Self { addr: addr.take_handle() as *mut (), size: AtomicUsize::new(0), capacity: capacity as usize}
    }

    fn get_address(&self,) -> symmetric_stream::Address {
        unsafe { Address::from_handle(self.addr as usize) }
    }

    fn get_size(&self,) -> u64 {
        self.size.load(Ordering::Relaxed) as u64
    }

    fn set_size(&self,size: u64,) -> () {
        self.size.store(size as usize, Ordering::Relaxed)
    }

    fn capacity(&self,) -> u64 {
        self.capacity as u64
    }
}

struct StreamObj {

}

impl GuestStreamObj for StreamObj {
    fn new() -> Self {
        todo!()
    }

    fn is_write_closed(&self,) -> bool {
        todo!()
    }

    fn start_reading(&self,buffer: symmetric_stream::Buffer,) -> () {
        todo!()
    }

    fn read_ready_event(&self,) -> symmetric_stream::EventGenerator {
        todo!()
    }

    fn read_result(&self,) -> symmetric_stream::Buffer {
        todo!()
    }

    fn close_read(stream: symmetric_stream::StreamObj,) -> () {
        todo!()
    }

    fn is_ready_to_write(&self,) -> bool {
        todo!()
    }

    fn write_ready_event(&self,) -> symmetric_stream::EventGenerator {
        todo!()
    }

    fn start_writing(&self,) -> symmetric_stream::Buffer {
        todo!()
    }

    fn finish_writing(&self,buffer: symmetric_stream::Buffer,) -> () {
        todo!()
    }

    fn close_write(stream: symmetric_stream::StreamObj,) -> () {
        todo!()
    }
}

const EOF_MARKER: usize = 1;

impl symmetric_stream::Guest for Guest {
    type Address = Dummy;

    type Buffer = Buffer;

    type StreamObj = StreamObj;

    fn end_of_file() -> symmetric_stream::Buffer {
        unsafe { symmetric_stream::Buffer::from_handle(EOF_MARKER) }
    }

    fn is_end_of_file(obj: symmetric_stream::BufferBorrow<'_>,) -> bool {
        let ptr: *mut () = unsafe { transmute(obj) };
        ptr as usize == EOF_MARKER
    }
}
