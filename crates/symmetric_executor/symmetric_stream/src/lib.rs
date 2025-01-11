use stream_impl::exports::symmetric::runtime::symmetric_stream::{self, GuestAddress, GuestBuffer, GuestStreamObj};

mod stream_impl;

struct Guest;

stream_impl::export!(Guest with_types_in stream_impl);

struct Dummy;

impl GuestAddress for Dummy {}

struct Buffer {
    addr: *mut (),
    size: usize,
}

impl GuestBuffer for Buffer {
    fn new(addr: symmetric_stream::Address,capacity: u64,) -> Self {
        todo!()
    }

    fn get_address(&self,) -> symmetric_stream::Address {
        todo!()
    }

    fn get_size(&self,) -> u64 {
        todo!()
    }

    fn set_size(&self,size: u64,) -> () {
        todo!()
    }

    fn capacity(&self,) -> u64 {
        todo!()
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

impl symmetric_stream::Guest for Guest {
    type Address = Dummy;

    type Buffer = Buffer;

    type StreamObj = StreamObj;

    fn end_of_file() -> symmetric_stream::Buffer {
        todo!()
    }

    fn is_end_of_file(obj: symmetric_stream::BufferBorrow<'_>,) -> bool {
        todo!()
    }
}
