use std::{
    mem::transmute,
    sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering},
};

use stream_impl::exports::symmetric::runtime::symmetric_stream::{
    self, Address, GuestAddress, GuestBuffer, GuestStreamObj,
};
use wit_bindgen_symmetric_rt::EventGenerator;

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
    fn new(addr: symmetric_stream::Address, capacity: u64) -> Self {
        Self {
            addr: addr.take_handle() as *mut (),
            size: AtomicUsize::new(0),
            capacity: capacity as usize,
        }
    }

    fn get_address(&self) -> symmetric_stream::Address {
        unsafe { Address::from_handle(self.addr as usize) }
    }

    fn get_size(&self) -> u64 {
        self.size.load(Ordering::Relaxed) as u64
    }

    fn set_size(&self, size: u64) -> () {
        self.size.store(size as usize, Ordering::Relaxed)
    }

    fn capacity(&self) -> u64 {
        self.capacity as u64
    }
}

mod results {
    pub const BLOCKED: isize = -1;
    pub const CLOSED: isize = isize::MIN;
    pub const CANCELED: isize = 0;
}

struct StreamObj {
    read_ready_event_send: *mut (),
    write_ready_event_send: *mut (),
    read_addr: AtomicPtr<()>,
    read_size: AtomicUsize,
    ready_size: AtomicIsize,
    active_instances: AtomicUsize,
}

impl GuestStreamObj for StreamObj {
    fn new() -> Self {
        Self {
            read_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            write_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            read_addr: AtomicPtr::new(core::ptr::null_mut()),
            read_size: AtomicUsize::new(0),
            ready_size: AtomicIsize::new(results::BLOCKED),
            active_instances: AtomicUsize::new(2),
        }
    }

    fn is_write_closed(&self) -> bool {
        self.ready_size.load(Ordering::Acquire) == results::CLOSED
    }

    fn start_reading(&self, buffer: symmetric_stream::Buffer) -> () {
        let buf = buffer.get().get_address().take_handle() as *mut ();
        let size = buffer.get().get_capacity();
        let old_ready = self.ready_size.load(Ordering::Acquire);
        if old_ready == results::CLOSED {
            return old_ready;
        }
        assert!(old_ready == results::BLOCKED);
        let old_size = self.read_size.swap(size, Ordering::Acquire);
        assert_eq!(old_size, 0);
        let old_ptr = self.read_addr.swap(buf, Ordering::Release);
        assert_eq!(old_ptr, std::ptr::null_mut());
        self.write_ready_event_send.activate();
        // unsafe { activate_event_send_ptr(write_evt) };
        results::BLOCKED
    }

    fn read_ready_event(&self) -> symmetric_stream::EventGenerator {
        unsafe {
            symmetric_stream::EventGenerator::from_handle(self.read_ready_event_send as usize)
        }
    }

    fn read_result(&self) -> symmetric_stream::Buffer {
        self.ready_size.swap(results::BLOCKED, Ordering::Acquire)
    }

    // fn close_read(stream: symmetric_stream::StreamObj) -> () {
    //     let refs = unsafe { &mut *stream }
    //         .active_instances
    //         .fetch_sub(1, Ordering::AcqRel);
    //     if refs == 1 {
    //         let obj = Box::from_raw(stream);
    //         drop(EventGenerator::from_handle(
    //             obj.read_ready_event_send as usize,
    //         ));
    //         drop(EventGenerator::from_handle(
    //             obj.write_ready_event_send as usize,
    //         ));
    //         drop(obj);
    //     }
    // }

    fn is_ready_to_write(&self) -> bool {
        self.read_addr.load(Ordering::Acquire).is_null()
    }

    fn write_ready_event(&self) -> symmetric_stream::EventGenerator {
        self.write_ready_event_send
    }

    fn start_writing(&self) -> symmetric_stream::Buffer {
        let size = self.read_size.swap(0, Ordering::Acquire);
        let addr = self
            .read_addr
            .swap(core::ptr::null_mut(), Ordering::Release);
        Buffer {
            addr,
            capacity: 0,
            size,
        }
        // Slice { addr, size }
    }

    fn finish_writing(&self, buffer: symmetric_stream::Buffer) -> () {
        let old_ready = self.ready_size.swap(elements as isize, Ordering::Release);
        assert_eq!(old_ready, results::BLOCKED);
        unsafe { activate_event_send_ptr(read_ready_event(stream)) };
    }

    // fn close_write(stream: symmetric_stream::StreamObj) -> () {
    //     todo!()
    // }
}

const EOF_MARKER: usize = 1;

impl symmetric_stream::Guest for Guest {
    type Address = Dummy;

    type Buffer = Buffer;

    type StreamObj = StreamObj;

    fn end_of_file() -> symmetric_stream::Buffer {
        unsafe { symmetric_stream::Buffer::from_handle(EOF_MARKER) }
    }

    fn is_end_of_file(obj: symmetric_stream::BufferBorrow<'_>) -> bool {
        let ptr: *mut () = unsafe { transmute(obj) };
        ptr as usize == EOF_MARKER
    }
}
