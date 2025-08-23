use std::{
    mem::transmute,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime},
};

use executor::exports::symmetric::runtime::symmetric_executor::{
    self, CallbackState, GuestCallbackRegistration,
};

const DEBUGGING: bool = cfg!(feature = "trace");
const INVALID_FD: EventFd = -1;

mod executor;

struct Guest;

executor::export!(Guest with_types_in executor);

struct Ignore;
struct OpaqueData;
impl symmetric_executor::GuestCallbackFunction for Ignore {}
impl symmetric_executor::GuestCallbackData for OpaqueData {}

// Hide the specifics of eventfd
mod event_fd {
    pub type EventFd = core::ffi::c_int;

    pub fn activate(fd: EventFd) {
        let file_signal: u64 = 1;
        if super::DEBUGGING {
            println!("activate(fd {fd})");
        }

        let result = unsafe {
            libc::write(
                fd,
                core::ptr::from_ref(&file_signal).cast(),
                core::mem::size_of_val(&file_signal),
            )
        };
        if result >= 0 {
            assert_eq!(
                result,
                core::mem::size_of_val(&file_signal).try_into().unwrap()
            );
        }
    }
    pub fn consume(fd: EventFd) {
        let mut dummy: u64 = 0;

        let readresult = unsafe {
            libc::read(
                fd,
                core::ptr::from_mut(&mut dummy).cast(),
                core::mem::size_of_val(&dummy),
            )
        };
        assert!(
            readresult <= 0
                || readresult == isize::try_from(core::mem::size_of_val(&dummy)).unwrap()
        );
    }
}

use event_fd::EventFd;

struct WaitSet {
    wait: libc::timeval,
    // non null if timeval is finite (timeout)
    tvptr: *mut libc::timeval,
    maxfd: EventFd,
    rfds: core::mem::MaybeUninit<libc::fd_set>,
}

struct WaitSetIterator<'a> {
    ws: &'a WaitSet,
    fd: EventFd,
}

impl<'a> Iterator for WaitSetIterator<'a> {
    type Item = EventFd;

    fn next(&mut self) -> Option<Self::Item> {
        let rfd_ptr = self.ws.rfds.as_ptr();
        while self.fd < self.ws.maxfd {
            let fd = self.fd;
            self.fd += 1;
            if unsafe { libc::FD_ISSET(fd, rfd_ptr) } {
                return Some(fd);
            }
        }
        None
    }
}

impl WaitSet {
    fn new(change_event: Option<EventFd>) -> Self {
        let wait = libc::timeval {
            tv_sec: i64::MAX,
            tv_usec: 999999,
        };
        let tvptr = core::ptr::null_mut();
        let maxfd = change_event.map_or(0, |fd| fd + 1);
        let mut rfds = core::mem::MaybeUninit::<libc::fd_set>::uninit();
        let rfd_ptr = rfds.as_mut_ptr();
        unsafe { libc::FD_ZERO(rfd_ptr) };
        if let Some(fd) = change_event {
            unsafe {
                libc::FD_SET(fd, rfd_ptr);
            }
        }
        Self {
            wait,
            tvptr,
            maxfd,
            rfds,
        }
    }

    fn register(&mut self, fd: EventFd) {
        let rfd_ptr = self.rfds.as_mut_ptr();
        unsafe { libc::FD_SET(fd, rfd_ptr) };
        if fd >= self.maxfd {
            self.maxfd = fd + 1;
        }
    }

    fn timeout(&mut self, diff: Duration) {
        let secs = diff.as_secs() as i64;
        let usecs = diff.subsec_micros() as i64;
        if secs < self.wait.tv_sec || (secs == self.wait.tv_sec && usecs < self.wait.tv_usec) {
            self.wait.tv_sec = secs;
            self.wait.tv_usec = usecs;
        }
        self.tvptr = core::ptr::from_mut(&mut self.wait);
    }

    fn debug(&self) {
        let rfd_ptr = self.rfds.as_ptr();
        if self.tvptr.is_null() {
            println!("select({}, {:x}, null)", self.maxfd, unsafe {
                *rfd_ptr.cast::<u32>()
            },);
        } else {
            println!(
                "select({}, {:x}, {}.{})",
                self.maxfd,
                unsafe { *rfd_ptr.cast::<u32>() },
                self.wait.tv_sec,
                self.wait.tv_usec
            );
        }
    }

    // see select for the return value
    fn wait(&mut self) -> i32 {
        let rfd_ptr = self.rfds.as_mut_ptr();
        unsafe {
            libc::select(
                self.maxfd,
                rfd_ptr,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                self.tvptr,
            )
        }
    }

    fn iter_active(&self) -> WaitSetIterator<'_> {
        WaitSetIterator { ws: self, fd: 0 }
    }
}

#[allow(dead_code)]
struct CallbackRegistrationInternal(usize);

impl symmetric_executor::GuestEventSubscription for EventSubscriptionInternal {
    fn ready(&self) -> bool {
        self.inner.ready()
    }

    fn from_timeout(nanoseconds: u64) -> symmetric_executor::EventSubscription {
        let when = SystemTime::now() + Duration::from_nanos(nanoseconds);
        symmetric_executor::EventSubscription::new(EventSubscriptionInternal {
            inner: EventType::SystemTime(when),
        })
    }

    fn dup(&self) -> symmetric_executor::EventSubscription {
        let res = symmetric_executor::EventSubscription::new(self.dup());
        // to avoid endless recursion de-activate the original
        self.reset();
        res
    }

    fn reset(&self) {
        match &self.inner {
            EventType::Triggered {
                last_counter,
                event,
            } => {
                last_counter.store(event.lock().unwrap().counter, Ordering::Relaxed);
            }
            EventType::SystemTime(_system_time) => (),
        }
    }
}

impl symmetric_executor::GuestEventGenerator for EventGenerator {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(EventInner {
            counter: 0,
            waiting: Default::default(),
        })))
    }

    fn subscribe(&self) -> symmetric_executor::EventSubscription {
        if DEBUGGING {
            println!("subscribe({:x})", Arc::as_ptr(&self.0) as usize);
        }
        symmetric_executor::EventSubscription::new(EventSubscriptionInternal {
            inner: EventType::Triggered {
                last_counter: AtomicU32::new(0),
                event: Arc::clone(&self.0),
            },
        })
    }

    fn activate(&self) {
        if let Ok(mut event) = self.0.lock() {
            event.counter += 1;
            if DEBUGGING {
                println!(
                    "activate({:x}) counter={}",
                    Arc::as_ptr(&self.0) as usize,
                    event.counter
                );
            }
            event.waiting.iter().for_each(|fd| {
                event_fd::activate(*fd);
            });
        } else if DEBUGGING {
            println!("activate failure");
        }
    }
}

impl GuestCallbackRegistration for CallbackRegistrationInternal {
    fn cancel(_obj: symmetric_executor::CallbackRegistration) -> symmetric_executor::CallbackData {
        todo!()
    }
}

struct Executor {
    active_tasks: Vec<QueuedEvent>,
    change_event: Option<EventFd>,
}

impl Executor {
    fn change_event(&mut self) -> EventFd {
        *self.change_event.get_or_insert_with(|| {
            let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK) };
            if DEBUGGING {
                println!("change event fd={fd}");
            }
            fd
        })
    }

    /// run the executor until it would block, returns number of handled events
    fn tick(ex: &mut std::sync::MutexGuard<'_, Self>, ws: &mut WaitSet) -> (usize, usize) {
        let mut count_events = 0;
        let mut count_waiting = 0;
        let now = SystemTime::now();
        let old_busy = EXECUTOR_BUSY.swap(true, Ordering::Acquire);
        assert!(!old_busy);
        ex.active_tasks.iter_mut().for_each(|task| {
            if task.inner.ready() {
                if DEBUGGING {
                    println!(
                        "task ready {:x} {:x}",
                        task.callback.as_ref().unwrap().0 as usize,
                        task.callback.as_ref().unwrap().1 as usize
                    );
                }
                count_events += 1;
                task.callback
                    .take_if(|CallbackEntry(f, data)| matches!((f)(*data), CallbackState::Ready));
            } else {
                count_waiting += 1;
                match &task.inner {
                    EventType::Triggered {
                        last_counter: _,
                        event: _,
                    } => {
                        ws.register(task.event_fd);
                    }
                    EventType::SystemTime(system_time) => {
                        if *system_time > now {
                            let diff = system_time.duration_since(now).unwrap_or_default();
                            ws.timeout(diff);
                        } else {
                            task.callback.take_if(|CallbackEntry(f, data)| {
                                matches!((f)(*data), CallbackState::Ready)
                            });
                        }
                    }
                }
            }
        });
        let old_busy = EXECUTOR_BUSY.swap(false, Ordering::Release);
        assert!(old_busy);
        ex.active_tasks.retain(|task| task.callback.is_some());
        (count_events, count_waiting)
    }
}

static EXECUTOR: Mutex<Executor> = Mutex::new(Executor {
    active_tasks: Vec::new(),
    change_event: None,
});
// while executing tasks from the loop we can't directly queue new ones
static EXECUTOR_BUSY: AtomicBool = AtomicBool::new(false);
static NEW_TASKS: Mutex<Vec<QueuedEvent>> = Mutex::new(Vec::new());

impl symmetric_executor::Guest for Guest {
    type CallbackFunction = Ignore;
    type CallbackData = OpaqueData;
    type CallbackRegistration = CallbackRegistrationInternal;
    type EventSubscription = EventSubscriptionInternal;
    type EventGenerator = EventGenerator;

    fn run() {
        let change_event = EXECUTOR.lock().unwrap().change_event();
        loop {
            let mut ws = WaitSet::new(Some(change_event));
            let (count_events, count_waiting) = {
                let mut ex = EXECUTOR.lock().unwrap();
                let (count_events, count_waiting) = Executor::tick(&mut ex, &mut ws);
                {
                    let mut new_tasks = NEW_TASKS.lock().unwrap();
                    if !new_tasks.is_empty() {
                        ex.active_tasks.append(&mut new_tasks);
                        // collect callbacks and timeouts again
                        continue;
                    }
                }
                if ex.active_tasks.is_empty() {
                    break;
                }
                (count_events, count_waiting)
            };
            if count_events != 0 {
                // we processed events, perhaps more became ready
                if DEBUGGING {
                    println!(
                        "Relooping with {} tasks after {count_events} events, {count_waiting} waiting",
                        EXECUTOR.lock().unwrap().active_tasks.len()
                    );
                }
                continue;
            }
            // with no work left the break should have occured
            // assert!(!tvptr.is_null() || maxfd > 0);
            if DEBUGGING {
                ws.debug();
            }
            let selectresult = ws.wait();
            // we could look directly for the timeout
            if selectresult > 0 {
                // reset active file descriptors
                for i in ws.iter_active() {
                    event_fd::consume(i);
                }
            }
        }
    }

    fn register(
        trigger: symmetric_executor::EventSubscription,
        callback: symmetric_executor::CallbackFunction,
        data: symmetric_executor::CallbackData,
    ) -> symmetric_executor::CallbackRegistration {
        let cb: CallbackType = unsafe { transmute(callback.take_handle()) };
        let data = data.take_handle() as *mut OpaqueData;

        // try to take a short cut
        let trigger: EventSubscriptionInternal = trigger.into_inner();
        if trigger.inner.ready() {
            if DEBUGGING {
                println!("register ready event {:x} {:x}", cb as usize, data as usize);
            }
            if matches!((cb)(data), CallbackState::Ready) {
                println!("registration unnecessary");
                return symmetric_executor::CallbackRegistration::new(
                    CallbackRegistrationInternal(0),
                );
            }
        }

        let subscr = QueuedEvent::new(trigger, CallbackEntry(cb, data));
        let id = subscr.id;
        match EXECUTOR.try_lock() {
            Ok(mut lock) => {
                lock.active_tasks.push(subscr);
                let mut ws = WaitSet::new(None);
                // process as long as there is immediate progress
                loop {
                    let (count_events, _count_waiting) = Executor::tick(&mut lock, &mut ws);
                    if count_events == 0 {
                        break;
                    }
                }
                // wake other threads last
                event_fd::activate(lock.change_event());
            }
            Err(_err) => {
                if EXECUTOR_BUSY.load(Ordering::Acquire) {
                    NEW_TASKS.lock().unwrap().push(subscr);
                } else {
                    // actually this is unlikely, but give it a try
                    EXECUTOR.lock().unwrap().active_tasks.push(subscr);
                }
            }
        }
        symmetric_executor::CallbackRegistration::new(CallbackRegistrationInternal(id))
    }

    fn block_on(trigger: symmetric_executor::EventSubscription) {
        let trigger: EventSubscriptionInternal = trigger.into_inner();
        // part of this function is never used
        let queue = QueuedEvent::new(
            trigger,
            CallbackEntry(
                unsafe { std::mem::transmute(std::ptr::null::<u8>()) },
                std::ptr::null_mut(),
            ),
        );
        let mut set = WaitSet::new(None);
        set.register(queue.event_fd);
        let active = set.wait();
        assert_eq!(active, queue.event_fd);
    }
}

type Count = u32;

struct EventInner {
    counter: Count,
    waiting: Vec<EventFd>,
}

struct EventGenerator(Arc<Mutex<EventInner>>);

type CallbackType = fn(*mut OpaqueData) -> CallbackState;
struct CallbackEntry(CallbackType, *mut OpaqueData);

unsafe impl Send for CallbackEntry {}

struct EventSubscriptionInternal {
    inner: EventType,
}

struct QueuedEvent {
    id: usize,
    inner: EventType,
    event_fd: EventFd,
    callback: Option<CallbackEntry>,
}

static ID_SOURCE: AtomicUsize = AtomicUsize::new(1);

impl QueuedEvent {
    fn new(trigger: EventSubscriptionInternal, callback: CallbackEntry) -> Self {
        let id = ID_SOURCE.fetch_add(1, Ordering::Relaxed);
        let inner = trigger.inner;
        let event_fd = match &inner {
            EventType::Triggered {
                last_counter: _,
                event,
            } => {
                let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK) };
                event.lock().unwrap().waiting.push(fd);
                fd
            }
            EventType::SystemTime(_system_time) => INVALID_FD,
        };
        if DEBUGGING {
            match &inner {
                EventType::Triggered {
                    last_counter: _,
                    event,
                } => println!(
                    "register(Trigger {:x} fd {event_fd}, {:x},{:x})",
                    Arc::as_ptr(event) as usize,
                    callback.0 as usize,
                    callback.1 as usize
                ),
                EventType::SystemTime(system_time) => {
                    let diff = match system_time.duration_since(SystemTime::now()) {
                        Ok(diff) => format!("{}.{}", diff.as_secs(), diff.subsec_nanos()),
                        Err(err) => format!("{err}"),
                    };
                    println!(
                        "register(Time {}, {:x},{:x})",
                        diff, callback.0 as usize, callback.1 as usize
                    );
                }
            }
        }
        QueuedEvent {
            id,
            inner,
            callback: Some(callback),
            event_fd,
        }
    }
}

impl EventSubscriptionInternal {
    fn dup(&self) -> Self {
        let inner = match &self.inner {
            EventType::Triggered {
                last_counter: last_counter_old,
                // event_fd,
                event,
            } => {
                let new_event = Arc::clone(event);
                let last_counter = last_counter_old.load(Ordering::Relaxed);
                if DEBUGGING {
                    println!(
                        "dup(subscr {last_counter} {:x})",
                        Arc::as_ptr(event) as usize
                    );
                }
                EventType::Triggered {
                    last_counter: AtomicU32::new(last_counter),
                    event: new_event,
                }
            }
            EventType::SystemTime(system_time) => EventType::SystemTime(*system_time),
        };
        EventSubscriptionInternal { inner }
    }
}

impl Drop for QueuedEvent {
    fn drop(&mut self) {
        if let Some(cb) = &self.callback {
            if DEBUGGING {
                println!(
                    "drop() with active callback {:x},{:x}",
                    cb.0 as usize, cb.1 as usize
                );
            }
        }
        match &self.inner {
            EventType::Triggered {
                last_counter: _,
                event,
            } => {
                if DEBUGGING {
                    println!("drop(queued fd {})", self.event_fd);
                }
                event
                    .lock()
                    .unwrap()
                    .waiting
                    .retain(|&e| e != self.event_fd);
                unsafe { libc::close(self.event_fd) };
            }
            EventType::SystemTime(_system_time) => (),
        }
    }
}

enum EventType {
    Triggered {
        last_counter: AtomicU32,
        event: Arc<Mutex<EventInner>>,
    },
    SystemTime(SystemTime),
}

impl EventType {
    pub fn ready(&self) -> bool {
        match self {
            EventType::Triggered {
                last_counter,
                event,
            } => {
                let current_counter = event.lock().unwrap().counter;
                let active = current_counter != last_counter.load(Ordering::Acquire);
                if active {
                    last_counter.store(current_counter, Ordering::Release);
                }
                active
            }
            EventType::SystemTime(system_time) => *system_time <= SystemTime::now(),
        }
    }
}
