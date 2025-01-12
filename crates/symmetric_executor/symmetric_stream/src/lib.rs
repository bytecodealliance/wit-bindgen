use std::{
    mem::transmute,
    ptr::null_mut,
    sync::{
        atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
};

use stream_impl::exports::symmetric::runtime::symmetric_stream::{
    self, Address, GuestAddress, GuestBuffer, GuestStreamObj,
};
use stream_impl::symmetric::runtime::symmetric_executor::{
    self, EventGenerator, EventSubscription,
};

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
    // pub const CLOSED: isize = isize::MIN;
    // pub const CANCELED: isize = 0;
}

struct StreamInner {
    read_ready_event_send: EventGenerator,
    write_ready_event_send: EventGenerator,
    read_addr: AtomicPtr<()>,
    read_size: AtomicUsize,
    ready_addr: AtomicPtr<()>,
    ready_size: AtomicIsize,
    ready_capacity: AtomicUsize,
}

struct StreamObj(Arc<StreamInner>);

impl GuestStreamObj for StreamObj {
    fn new() -> Self {
        let inner = StreamInner {
            read_ready_event_send: EventGenerator::new(),
            write_ready_event_send: EventGenerator::new(),
            read_addr: AtomicPtr::new(core::ptr::null_mut()),
            read_size: AtomicUsize::new(0),
            ready_addr: AtomicPtr::new(core::ptr::null_mut()),
            ready_size: AtomicIsize::new(results::BLOCKED),
            ready_capacity: AtomicUsize::new(0),
        };
        Self(Arc::new(inner))
    }

    fn is_write_closed(&self) -> bool {
        self.0.ready_addr.load(Ordering::Acquire) as usize == EOF_MARKER
    }

    fn start_reading(&self, buffer: symmetric_stream::Buffer) {
        let buf = buffer.get::<Buffer>().get_address().take_handle() as *mut ();
        let size = buffer.get::<Buffer>().capacity();
        let old_readya = self.0.ready_addr.load(Ordering::Acquire);
        let old_ready = self.0.ready_size.load(Ordering::Acquire);
        if old_readya as usize == EOF_MARKER {
            todo!();
            // return old_ready;
        }
        assert!(old_ready == results::BLOCKED);
        let old_size = self.0.read_size.swap(size as usize, Ordering::Acquire);
        assert_eq!(old_size, 0);
        let old_ptr = self.0.read_addr.swap(buf, Ordering::Release);
        assert_eq!(old_ptr, std::ptr::null_mut());
        self.write_ready_activate();
        // unsafe { activate_event_send_ptr(write_evt) };
        // results::BLOCKED
    }

    // fn read_ready_event(&self) -> symmetric_stream::EventGenerator {
    //     unsafe {
    //         symmetric_stream::EventGenerator::from_handle(self.read_ready_event_send as usize)
    //     }
    // }

    fn read_result(&self) -> Option<symmetric_stream::Buffer> {
        let size = self.0.ready_size.swap(results::BLOCKED, Ordering::Acquire);
        let addr = self.0.ready_addr.swap(null_mut(), Ordering::Acquire);
        if addr as usize == EOF_MARKER {
            None
        } else {
            Some(symmetric_stream::Buffer::new(Buffer {
                addr,
                capacity: size as usize,
                size: AtomicUsize::new(size as usize),
            }))
        }
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
        !self.0.read_addr.load(Ordering::Acquire).is_null()
    }

    // fn write_ready_event(&self) -> symmetric_stream::EventGenerator {
    //     //self.write_ready_event_send.clone()
    //     unsafe {
    //         symmetric_stream::EventGenerator::from_handle(self.write_ready_event_send as usize)
    //     }
    // }

    fn start_writing(&self) -> symmetric_stream::Buffer {
        let size = self.0.read_size.swap(0, Ordering::Acquire);
        let addr = self
            .0
            .read_addr
            .swap(core::ptr::null_mut(), Ordering::Release);
        symmetric_stream::Buffer::new(Buffer {
            addr,
            capacity: size,
            size: AtomicUsize::new(0),
        })
    }

    fn finish_writing(&self, buffer: Option<symmetric_stream::Buffer>) -> () {
        let (elements, addr) = if let Some(buffer) = buffer {
            let elements = buffer.get::<Buffer>().get_size() as isize;
            let addr = buffer.get::<Buffer>().get_address().take_handle() as *mut ();
            (elements, addr)
        } else {
            (0, EOF_MARKER as usize as *mut ())
        };
        let old_ready = self.0.ready_size.swap(elements as isize, Ordering::Relaxed);
        let _old_readya = self.0.ready_addr.swap(addr, Ordering::Release);
        assert_eq!(old_ready, results::BLOCKED);
        self.read_ready_activate();
        // unsafe { activate_event_send_ptr(self.read_ready_event) };
    }

    fn clone(&self) -> symmetric_stream::StreamObj {
        // let _= self.0.active_instances.fetch_add(1, Ordering::AcqRel);
        symmetric_stream::StreamObj::new(StreamObj(Arc::clone(&self.0)))
    }

    fn write_ready_activate(&self) {
        self.0.write_ready_event_send.activate();
    }

    fn read_ready_subscribe(&self) -> symmetric_stream::EventSubscription {
        self.0.read_ready_event_send.subscribe()
    }

    fn write_ready_subscribe(&self) -> symmetric_stream::EventSubscription {
        self.0.write_ready_event_send.subscribe()
    }

    fn read_ready_activate(&self) {
        self.0.read_ready_event_send.activate();
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
}
