use std::{
    ffi::c_int,
    mem::transmute,
    sync::{atomic::AtomicU32, Arc, Mutex},
    time::{Duration, SystemTime},
};

use executor::exports::symmetric::runtime::symmetric_executor::{
    self, CallbackData, CallbackState, GuestEventSubscription,
};

mod executor;

struct Guest;

executor::export!(Guest with_types_in executor);

struct Ignore;
impl symmetric_executor::GuestCallbackFunction for Ignore {}
impl symmetric_executor::GuestCallbackData for Ignore {}

impl symmetric_executor::GuestEventSubscription for EventSubscription {
    fn ready(&self) -> bool {
        match &self.inner {
            EventType::Triggered {
                last_counter,
                event_fd: _,
                event,
            } => {
                let current_counter = event.lock().unwrap().counter;
                let active =
                    current_counter != last_counter.load(std::sync::atomic::Ordering::Acquire);
                if active {
                    last_counter.store(current_counter, std::sync::atomic::Ordering::Release);
                }
                active
            }
            EventType::SystemTime(system_time) => *system_time <= SystemTime::now(),
        }
    }

    fn from_timeout(nanoseconds: u64) -> symmetric_executor::EventSubscription {
        let when = SystemTime::now() + Duration::from_nanos(nanoseconds);
        symmetric_executor::EventSubscription::new(EventSubscription {
            inner: EventType::SystemTime(when),
            callback: None,
        })
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
        let event_fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK) };
        self.0.lock().unwrap().waiting.push(event_fd);
        symmetric_executor::EventSubscription::new(EventSubscription {
            inner: EventType::Triggered {
                last_counter: AtomicU32::new(0),
                event_fd,
                event: Arc::clone(&self.0),
            },
            callback: None,
        })
    }

    fn activate(&self) {
        if let Ok(mut event) = self.0.lock() {
            event.counter += 1;
            let file_signal: u64 = 1;
            event.waiting.iter().for_each(|fd| {
                let result = unsafe {
                    libc::write(
                        *fd,
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
            });
        }
    }
}

struct Executor {
    active_tasks: Vec<EventSubscription>,
}

static EXECUTOR: Mutex<Executor> = Mutex::new(Executor {
    active_tasks: Vec::new(),
});

impl symmetric_executor::Guest for Guest {
    type CallbackFunction = Ignore;
    type CallbackData = Ignore;
    type EventSubscription = EventSubscription;
    type EventGenerator = EventGenerator;

    fn run() {
        loop {
            let mut wait = libc::timeval {
                tv_sec: i64::MAX,
                tv_usec: 999999,
            };
            let mut tvptr = core::ptr::null_mut();
            let mut maxfd = 0;
            let now = SystemTime::now();
            let mut rfds = core::mem::MaybeUninit::<libc::fd_set>::uninit();
            let rfd_ptr = unsafe { core::ptr::from_mut(rfds.assume_init_mut()) };
            unsafe { libc::FD_ZERO(rfd_ptr) };
            {
                let mut ex = EXECUTOR.lock().unwrap();
                ex.active_tasks.iter_mut().for_each(|task| {
                    if task.ready() {
                        task.callback.take_if(|(f, data)| {
                            matches!((f)(data.handle() as *mut ()), CallbackState::Ready)
                        });
                    } else {
                        match &task.inner {
                            EventType::Triggered {
                                last_counter: _,
                                event_fd,
                                event: _,
                            } => {
                                unsafe { libc::FD_SET(*event_fd, rfd_ptr) };
                                if *event_fd > maxfd {
                                    maxfd = *event_fd + 1;
                                }
                            }
                            EventType::SystemTime(system_time) => {
                                if *system_time > now {
                                    let diff = system_time.duration_since(now).unwrap_or_default(); //.as_micros();
                                    let secs = diff.as_secs() as i64;
                                    let usecs = diff.subsec_micros() as i64;
                                    if secs < wait.tv_sec
                                        || (secs == wait.tv_sec && usecs < wait.tv_usec)
                                    {
                                        wait.tv_sec = secs;
                                        wait.tv_usec = usecs;
                                        // timeoutindex = n;
                                    }
                                    tvptr = core::ptr::from_mut(&mut wait);
                                } else {
                                    task.callback.take_if(|(f, data)| {
                                        matches!(
                                            (f)(data.handle() as *mut ()),
                                            CallbackState::Ready
                                        )
                                    });
                                }
                            }
                        }
                    }
                });
                ex.active_tasks.retain(|task| task.callback.is_some());
                if ex.active_tasks.is_empty() {
                    break;
                }
            }
            // with no work left the break should have occured
            assert!(!tvptr.is_null() || maxfd > 0);
            let selectresult = unsafe {
                libc::select(
                    maxfd,
                    rfd_ptr,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    tvptr,
                )
            };
            // we could look directly for the timeout
            if selectresult > 0 {
                let mut dummy: u64 = 0;
                // reset active file descriptors
                for i in 0..maxfd {
                    if unsafe { libc::FD_ISSET(i, rfd_ptr) } {
                        let readresult = unsafe {
                            libc::read(
                                i,
                                core::ptr::from_mut(&mut dummy).cast(),
                                core::mem::size_of_val(&dummy),
                            )
                        };
                        assert!(
                            readresult <= 0
                                || readresult
                                    == isize::try_from(core::mem::size_of_val(&dummy)).unwrap()
                        );
                    }
                }
            }
        }
    }

    fn register(
        trigger: symmetric_executor::EventSubscription,
        callback: symmetric_executor::CallbackFunction,
        data: symmetric_executor::CallbackData,
    ) -> () {
        // TODO: Tidy this mess up
        let mut subscr = EventSubscription {
            inner: EventType::SystemTime(std::time::UNIX_EPOCH),
            callback: None,
        };
        std::mem::swap(&mut subscr, unsafe {
            &mut *(trigger.take_handle() as *mut EventSubscription)
        });
        let cb: fn(*mut ()) -> CallbackState = unsafe { transmute(callback.take_handle()) };
        subscr.callback.replace((cb, data));
        EXECUTOR.lock().unwrap().active_tasks.push(subscr);
    }
}

type EventFd = c_int;
type Count = u32;

struct EventInner {
    counter: Count,
    waiting: Vec<EventFd>,
}

struct EventGenerator(Arc<Mutex<EventInner>>);

type CallbackEntry = (fn(*mut ()) -> CallbackState, CallbackData);

struct EventSubscription {
    inner: EventType,
    callback: Option<CallbackEntry>,
}

enum EventType {
    Triggered {
        last_counter: AtomicU32,
        event_fd: EventFd,
        event: Arc<Mutex<EventInner>>,
    },
    SystemTime(SystemTime),
}
