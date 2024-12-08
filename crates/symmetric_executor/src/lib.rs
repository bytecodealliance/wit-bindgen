use std::{
    ffi::c_int,
    sync::{atomic::AtomicU32, Arc, Mutex}, time::{Duration, SystemTime},
};

use executor::exports::symmetric::runtime::symmetric_executor::{self, CallbackData, CallbackState};

mod executor;

struct Guest;

executor::export!(Guest with_types_in executor);

struct Ignore;
impl symmetric_executor::GuestCallbackFunction for Ignore {}
impl symmetric_executor::GuestCallbackData for Ignore {}

impl symmetric_executor::GuestEventSubscription for EventSubscription {
    fn ready(&self) -> bool {
        match &self.inner {
            EventType::Triggered { last_counter: _, event_fd: _, object: _ } => todo!(),
            EventType::SystemTime(system_time) => *system_time <= SystemTime::now(),
        }
    }

    fn from_timeout(nanoseconds: u64) -> symmetric_executor::EventSubscription {
        let when = SystemTime::now() + Duration::from_nanos(nanoseconds);
        symmetric_executor::EventSubscription::new(EventSubscription{ inner: EventType::SystemTime(when), callback: None })
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
    type CallbackFunction = Ignore;
    type CallbackData = Ignore;
    type EventSubscription = EventSubscription;
    type EventGenerator = EventGenerator;

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

struct EventGenerator {}

type CallbackEntry = (fn(*mut ()) -> CallbackState, CallbackData);

struct EventSubscription {
    inner: EventType,
    callback: Option<CallbackEntry>,
}

enum EventType {
    Triggered {
        last_counter: AtomicU32,
        event_fd: EventFd,
        object: Arc<Mutex<EventInner>>,
    },
    SystemTime(SystemTime),
}
