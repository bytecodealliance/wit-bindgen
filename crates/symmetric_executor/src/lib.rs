use std::{
    ffi::c_int,
    mem::transmute,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime},
};

use executor::exports::symmetric::runtime::symmetric_executor::{
    self, CallbackState, GuestEventSubscription,
};

const DEBUGGING: bool = true;

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
                let active = current_counter != last_counter.load(Ordering::Acquire);
                if active {
                    last_counter.store(current_counter, Ordering::Release);
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
        if DEBUGGING {
            println!(
                "subscribe({:x}) fd={event_fd}",
                Arc::as_ptr(&self.0) as usize
            );
        }
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
        if DEBUGGING {
            println!("activate({:x})", Arc::as_ptr(&self.0) as usize);
        }
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
// while executing tasks from the loop we can't directly queue new ones
static EXECUTOR_BUSY: AtomicBool = AtomicBool::new(false);
static NEW_TASKS: Mutex<Vec<EventSubscription>> = Mutex::new(Vec::new());

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
                let old_busy = EXECUTOR_BUSY.swap(true, Ordering::SeqCst);
                assert!(!old_busy);
                ex.active_tasks.iter_mut().for_each(|task| {
                    if task.ready() {
                        if DEBUGGING {
                            println!(
                                "task ready {:x} {:x}",
                                task.callback.as_ref().unwrap().0 as usize,
                                task.callback.as_ref().unwrap().1 as usize
                            );
                        }
                        task.callback.take_if(|CallbackEntry(f, data)| {
                            matches!((f)(*data), CallbackState::Ready)
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
                                    task.callback.take_if(|CallbackEntry(f, data)| {
                                        matches!((f)(*data), CallbackState::Ready)
                                    });
                                }
                            }
                        }
                    }
                });
                let old_busy = EXECUTOR_BUSY.swap(false, Ordering::SeqCst);
                assert!(old_busy);
                ex.active_tasks.retain(|task| task.callback.is_some());
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
            }
            // with no work left the break should have occured
            assert!(!tvptr.is_null() || maxfd > 0);
            if DEBUGGING {
                if tvptr.is_null() {
                    println!("select({maxfd}, {:x}, null)", unsafe {
                        *rfd_ptr.cast::<u32>()
                    },);
                } else {
                    println!(
                        "select({maxfd}, {:x}, {}.{})",
                        unsafe { *rfd_ptr.cast::<u32>() },
                        wait.tv_sec,
                        wait.tv_usec
                    );
                }
            }
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
        // Note: Trigger is consumed, callback and data are managed elsewhere
        let mut subscr = EventSubscription {
            inner: EventType::SystemTime(std::time::UNIX_EPOCH),
            callback: None,
        };
        std::mem::swap(&mut subscr, unsafe {
            // TODO: This should be handle to free the resource, but it is used later?
            &mut *(trigger.take_handle() as *mut EventSubscription)
        });
        if DEBUGGING {
            match &subscr.inner {
                EventType::Triggered {
                    last_counter: _,
                    event_fd,
                    event,
                } => println!(
                    "register(Triggered {:x} fd {event_fd}, {:x},{:x})",
                    Arc::as_ptr(event) as usize,
                    callback.handle(),
                    data.handle()
                ),
                EventType::SystemTime(system_time) => {
                    let diff = system_time.duration_since(SystemTime::now()).unwrap();
                    println!(
                        "register(Time {}.{}, {:x},{:x})",
                        diff.as_secs(),
                        diff.subsec_nanos(),
                        callback.handle(),
                        data.handle()
                    );
                }
            }
        }
        let cb: fn(*mut ()) -> CallbackState = unsafe { transmute(callback.take_handle()) };
        let data = data.take_handle() as *mut ();
        subscr.callback.replace(CallbackEntry(cb, data));
        match EXECUTOR.try_lock() {
            Ok(mut lock) => lock.active_tasks.push(subscr),
            Err(_err) => {
                if EXECUTOR_BUSY.load(Ordering::Relaxed) {
                    NEW_TASKS.lock().unwrap().push(subscr);
                } else {
                    // actually this is unlikely, but give it a try
                    EXECUTOR.lock().unwrap().active_tasks.push(subscr);
                }
            }
        }
    }
}

type EventFd = c_int;
type Count = u32;

struct EventInner {
    counter: Count,
    waiting: Vec<EventFd>,
}

struct EventGenerator(Arc<Mutex<EventInner>>);

struct CallbackEntry(fn(*mut ()) -> CallbackState, *mut ());

unsafe impl Send for CallbackEntry {}

struct EventSubscription {
    inner: EventType,
    callback: Option<CallbackEntry>,
}

impl EventSubscription {
    fn dup(&self) -> Self {
        let inner = match &self.inner {
            EventType::Triggered {
                last_counter: last_counter_old,
                event_fd,
                event,
            } => {
                let new_event_fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK) };
                let new_event = Arc::clone(event);
                let last_counter = last_counter_old.load(Ordering::Relaxed);
                if DEBUGGING {
                    println!(
                        "dup(subscr {last_counter} {event_fd} {:x}) fd={new_event_fd}",
                        Arc::as_ptr(&event) as usize
                    );
                }
                new_event.lock().unwrap().waiting.push(new_event_fd);
                EventType::Triggered {
                    last_counter: AtomicU32::new(last_counter),
                    event_fd: new_event_fd,
                    event: new_event,
                }
            }
            EventType::SystemTime(system_time) => EventType::SystemTime(*system_time),
        };
        EventSubscription {
            inner,
            callback: None,
        }
    }
}

impl Drop for EventSubscription {
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
                event_fd,
                event,
            } => {
                if DEBUGGING {
                    println!("drop(subscription fd {event_fd}");
                }
                event.lock().unwrap().waiting.retain(|e| e != event_fd);
                unsafe { libc::close(*event_fd) };
            }
            EventType::SystemTime(_system_time) => (),
        }
    }
}

enum EventType {
    Triggered {
        last_counter: AtomicU32,
        event_fd: EventFd,
        event: Arc<Mutex<EventInner>>,
    },
    SystemTime(SystemTime),
}
