use std::{ffi::c_int, sync::{atomic::AtomicU32, Arc, Mutex}};

use executor::exports::symmetric::runtime::symmetric_executor;

mod executor;

struct Guest;

executor::export!(Guest with_types_in executor);

struct Ignore;
impl symmetric_executor::GuestCallbackFunction for Ignore {}
impl symmetric_executor::GuestCallbackData for Ignore {}

impl symmetric_executor::GuestEventSubscription for EventSubscription {
    fn ready(&self) -> bool {
        todo!()
    }

    fn from_timeout(nanoseconds: u64) -> symmetric_executor::EventSubscription {
        todo!()
    }
}

impl symmetric_executor::GuestEventGenerator for EventGenerator {
    fn new() -> Self {
        todo!()
    }

    fn subscribe(&self) -> symmetric_executor::EventSubscription {
        todo!()
    }

    fn activate(&self) -> () {
        todo!()
    }
}

impl symmetric_executor::Guest for Guest {
    type CallbackFunction=Ignore;
    type CallbackData=Ignore;
    type EventSubscription = EventSubscription;
    type EventGenerator= EventGenerator;

    fn run() -> () {
        todo!()
    }

    fn register(
        trigger: symmetric_executor::EventSubscription,
        callback: symmetric_executor::CallbackFunction,
        data: symmetric_executor::CallbackData,
    ) -> () {
        todo!()
    }
}

type EventFd = c_int;
type Count = u32;

struct EventInner {
    counter: Count,
    waiting: Vec<EventFd>,
}

struct EventGenerator {

}

struct EventSubscription {

}

enum EventType {
    Manual {
        last_counter: AtomicU32,
        event_fd: EventFd,
        object: Arc<Mutex<EventInner>>,
    }
}
