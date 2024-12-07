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

    fn from_timeout(_nanoseconds: u64) -> symmetric_executor::EventSubscription {
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
        _trigger: symmetric_executor::EventSubscription,
        _callback: symmetric_executor::CallbackFunction,
        _data: symmetric_executor::CallbackData,
    ) -> () {
        todo!()
    }
}

struct EventGenerator {

}

struct EventSubscription {

}
