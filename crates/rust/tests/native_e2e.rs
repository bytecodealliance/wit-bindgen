//! End-to-end test for native (non-wasm) linking. Builds the plugin in
//! `tests/native-e2e` as a `cdylib`, `dlopen`s it, checks the world marker,
//! installs an import resolver, and calls its exports through the core ABI.
//! The sync and async parts of the world have separate tests that share the
//! loaded library.

#![cfg(not(target_arch = "wasm32"))]

use libloading::{Library, Symbol};
use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::ffi::{CStr, c_char};
use std::mem::size_of;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};
use wit_bindgen_core::symbol_name::make_external_component;

const WORLD: &str = "test:native-e2e/plugin";
const HOST: &str = "test:native-e2e/host";

/// Arbitrary non-null context passed to `__wit_bindgen_set_import_resolver`.
/// The guest never dereferences it; it only passes it back to the resolver,
/// which checks that it did. A real host would pass a pointer to its state.
const CTX: *mut () = 0x1234 as *mut ();

type Realloc = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
type ImportResolver = unsafe extern "C" fn(*mut (), *const c_char, *const c_char) -> *mut ();
type SetImportResolver = unsafe extern "C" fn(Option<ImportResolver>, *mut ());

/// The plugin's allocator, saved at load time so imports can allocate guest
/// memory.
static REALLOC: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

/// Copies `bytes` into guest memory. The guest takes ownership of the
/// allocation.
unsafe fn lower_bytes(bytes: &[u8], align: usize) -> *mut u8 {
    let realloc: Realloc = unsafe { std::mem::transmute(REALLOC.load(Ordering::Acquire)) };
    let dst = unsafe { realloc(ptr::null_mut(), 0, align, bytes.len()) };
    assert!(!dst.is_null());
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len()) };
    dst
}

// Core signatures: `u32` is `i32`, `string` is `(*mut u8, usize)`, and a
// resource handle is an `i32` index into a host-side table.
mod sync_host {
    use super::HOST;
    use std::sync::Mutex;

    pub static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    pub static COUNTERS: Mutex<Counters> = Mutex::new(Counters(Vec::new()));

    unsafe extern "C" fn log(ptr: *mut u8, len: usize) {
        let msg = unsafe { std::slice::from_raw_parts(ptr, len) };
        LOG.lock()
            .unwrap()
            .push(String::from_utf8(msg.to_vec()).unwrap());
    }

    unsafe extern "C" fn add(a: i32, b: i32) -> i32 {
        (a as u32).wrapping_add(b as u32) as i32
    }

    /// Table of `counter` resources. Dropped counters stay as `None` so a
    /// double drop is caught.
    ///
    /// Handles are 1-based; the guest's `Resource` reserves `0` and `u32::MAX`.
    pub struct Counters(Vec<Option<u32>>);

    impl Counters {
        fn create(&mut self, start: u32) -> i32 {
            self.0.push(Some(start));
            self.0.len() as i32
        }

        fn bump(&mut self, handle: i32) -> u32 {
            let value = self
                .slot(handle)
                .as_mut()
                .expect("bump on a dropped counter");
            *value += 1;
            *value
        }

        fn release(&mut self, handle: i32) {
            assert!(self.slot(handle).take().is_some(), "counter dropped twice");
        }

        fn slot(&mut self, handle: i32) -> &mut Option<u32> {
            &mut self.0[handle as usize - 1]
        }

        /// Number of counters created.
        pub fn created(&self) -> usize {
            self.0.len()
        }

        /// Number of counters not yet dropped.
        pub fn live(&self) -> usize {
            self.0.iter().filter(|c| c.is_some()).count()
        }

        // Core-ABI entry points for `[constructor]counter`,
        // `[method]counter.bump`, and `[resource-drop]counter`.
        unsafe extern "C" fn abi_new(start: i32) -> i32 {
            COUNTERS.lock().unwrap().create(start as u32)
        }

        unsafe extern "C" fn abi_bump(handle: i32) -> i32 {
            COUNTERS.lock().unwrap().bump(handle) as i32
        }

        unsafe extern "C" fn abi_drop(handle: i32) {
            COUNTERS.lock().unwrap().release(handle)
        }
    }

    pub fn resolve(module: &str, name: &str) -> Option<*mut ()> {
        Some(match (module, name) {
            (HOST, "log") => log as *mut (),
            (HOST, "add") => add as *mut (),
            (HOST, "[constructor]counter") => Counters::abi_new as *mut (),
            (HOST, "[method]counter.bump") => Counters::abi_bump as *mut (),
            (HOST, "[resource-drop]counter") => Counters::abi_drop as *mut (),
            _ => return None,
        })
    }
}

// A minimal host runtime: enough of the async intrinsics for one async export
// that awaits one async import and writes one stream, with the host completing
// everything synchronously. Intrinsics the test does not expect are wired to
// functions that panic with their name.
mod async_host {
    use super::HOST;
    use std::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicPtr, Ordering};

    // Encodings from `rt::async_support`.
    const STATUS_RETURNED: u32 = 2;
    const COMPLETED: u32 = 0x0;

    #[derive(Debug, Clone, PartialEq)]
    pub enum Returned {
        Ok { reader: u32, kind: u32, value: i64 },
        Err(String),
    }

    pub static RETURNED: Mutex<Vec<Returned>> = Mutex::new(Vec::new());
    pub static STREAMS: Mutex<Streams> = Mutex::new(Streams(Vec::new()));
    static CONTEXT: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());

    /// Table of streams, one per `stream.new`. Writes are buffered so they
    /// complete immediately; otherwise the guest could not write before
    /// returning the reader.
    ///
    /// Handles are non-zero: reader = 2i + 1, writer = 2i + 2.
    pub struct Streams(Vec<Stream>);

    pub struct Stream {
        pub buf: Vec<u8>,
        pub writer_dropped: bool,
    }

    impl Streams {
        fn create(&mut self) -> u64 {
            self.0.push(Stream {
                buf: Vec::new(),
                writer_dropped: false,
            });
            let i = (self.0.len() - 1) as u64;
            let reader = 2 * i + 1;
            let writer = 2 * i + 2;
            (writer << 32) | reader
        }

        fn write(&mut self, handle: u32, bytes: &[u8]) -> u32 {
            self.slot(handle).buf.extend_from_slice(bytes);
            ((bytes.len() as u32) << 4) | COMPLETED
        }

        fn release_writer(&mut self, handle: u32) {
            self.slot(handle).writer_dropped = true;
        }

        fn slot(&mut self, handle: u32) -> &mut Stream {
            &mut self.0[(handle as usize - 1) / 2]
        }

        /// Number of streams created.
        pub fn created(&self) -> usize {
            self.0.len()
        }

        /// The stream for either of its handles.
        pub fn get(&self, handle: u32) -> &Stream {
            &self.0[(handle as usize - 1) / 2]
        }

        // Core-ABI entry points for `[stream-new-0]produce`,
        // `[async-lower][stream-write-0]produce`, and
        // `[stream-drop-writable-0]produce`.
        unsafe extern "C" fn abi_new() -> u64 {
            STREAMS.lock().unwrap().create()
        }

        unsafe extern "C" fn abi_write(handle: u32, ptr: *const u8, amt: usize) -> u32 {
            let bytes = unsafe { std::slice::from_raw_parts(ptr, amt) };
            STREAMS.lock().unwrap().write(handle, bytes)
        }

        unsafe extern "C" fn abi_drop_writable(handle: u32) {
            STREAMS.lock().unwrap().release_writer(handle)
        }
    }

    // `fetch: async func(id: s64) -> result<tuple<kind, s64>, string>`.
    // Writes the result to `results` (discriminant at 0, payload at 8 and
    // 16) and completes before returning, so the status is `RETURNED` with
    // no subtask handle.
    unsafe extern "C" fn fetch(id: i64, results: *mut u8) -> i32 {
        unsafe {
            if id >= 0 {
                let kind: u8 = if id < 10 { 0 } else { 1 };
                *results = 0;
                *results.add(8) = kind;
                *(results.add(16) as *mut i64) = id + 1;
            } else {
                let msg = "negative id";
                *results = 1;
                *(results.add(8) as *mut *mut u8) = super::lower_bytes(msg.as_bytes(), 1);
                *(results.add(16) as *mut usize) = msg.len();
            }
        }
        STATUS_RETURNED as i32
    }

    // `[task-return]produce` takes the flattened
    // `result<tuple<stream<u8>, kind, s64>, string>`. The ok and err arms
    // share slots: slot 1 is a pointer holding either the stream handle or
    // the string pointer, slot 2 a `usize` holding either the enum
    // discriminant or the string length.
    unsafe extern "C" fn task_return(disc: i32, a1: *mut u8, a2: usize, a3: i64) {
        let returned = match disc {
            0 => Returned::Ok {
                reader: a1 as usize as u32,
                kind: a2 as u32,
                value: a3,
            },
            1 => {
                let bytes = unsafe { std::slice::from_raw_parts(a1, a2) };
                Returned::Err(String::from_utf8(bytes.to_vec()).unwrap())
            }
            _ => panic!("bad result discriminant {disc}"),
        };
        RETURNED.lock().unwrap().push(returned);
    }

    unsafe extern "C" fn context_get() -> *mut u8 {
        CONTEXT.load(Ordering::Acquire)
    }

    unsafe extern "C" fn context_set(value: *mut u8) {
        CONTEXT.store(value, Ordering::Release);
    }

    // Intrinsics the guest can reach but this test does not expect.
    macro_rules! unexpected {
        ($($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)*) => {$(
            unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
                $(let _ = $arg;)*
                panic!(concat!("guest called `", stringify!($name), "`, which this test does not expect"))
            }
        )*};
    }
    unexpected! {
        waitable_set_new() -> u32;
        waitable_set_drop(set: u32);
        waitable_join(waitable: u32, set: u32);
        waitable_set_wait(set: u32, event: *mut u32) -> u32;
        waitable_set_poll(set: u32, event: *mut u32) -> u32;
        subtask_drop(handle: u32);
        subtask_cancel(handle: u32) -> u32;
        task_cancel();
        thread_yield() -> bool;
        backpressure_inc();
        backpressure_dec();
        stream_read(handle: u32, ptr: *mut u8, amt: usize) -> u32;
        stream_cancel_write(handle: u32) -> u32;
        stream_cancel_read(handle: u32) -> u32;
        stream_drop_readable(handle: u32);
    }

    pub fn resolve(module: &str, name: &str) -> Option<*mut ()> {
        Some(match (module, name) {
            (HOST, "[async-lower]fetch") => fetch as *mut (),

            ("$root", "[context-get-0]") => context_get as *mut (),
            ("$root", "[context-set-0]") => context_set as *mut (),
            ("$root", "[waitable-set-new]") => waitable_set_new as *mut (),
            ("$root", "[waitable-set-drop]") => waitable_set_drop as *mut (),
            ("$root", "[waitable-join]") => waitable_join as *mut (),
            ("$root", "[waitable-set-wait]") => waitable_set_wait as *mut (),
            ("$root", "[waitable-set-poll]") => waitable_set_poll as *mut (),
            ("$root", "[subtask-drop]") => subtask_drop as *mut (),
            ("$root", "[subtask-cancel]") => subtask_cancel as *mut (),
            ("$root", "[thread-yield]") => thread_yield as *mut (),
            ("$root", "[backpressure-inc]") => backpressure_inc as *mut (),
            ("$root", "[backpressure-dec]") => backpressure_dec as *mut (),

            ("[export]$root", "[task-return]produce") => task_return as *mut (),
            ("[export]$root", "[task-cancel]") => task_cancel as *mut (),
            ("[export]$root", "[stream-new-0]produce") => Streams::abi_new as *mut (),
            ("[export]$root", "[async-lower][stream-write-0]produce") => {
                Streams::abi_write as *mut ()
            }
            ("[export]$root", "[async-lower][stream-read-0]produce") => stream_read as *mut (),
            ("[export]$root", "[stream-cancel-write-0]produce") => stream_cancel_write as *mut (),
            ("[export]$root", "[stream-cancel-read-0]produce") => stream_cancel_read as *mut (),
            ("[export]$root", "[stream-drop-writable-0]produce") => {
                Streams::abi_drop_writable as *mut ()
            }
            ("[export]$root", "[stream-drop-readable-0]produce") => stream_drop_readable as *mut (),

            _ => return None,
        })
    }
}

// Host-side

unsafe extern "C" fn resolver(ctx: *mut (), module: *const c_char, name: *const c_char) -> *mut () {
    assert_eq!(ctx, CTX, "resolver was handed the wrong context");
    let module = unsafe { CStr::from_ptr(module) }.to_str().unwrap();
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    sync_host::resolve(module, name)
        .or_else(|| async_host::resolve(module, name))
        .unwrap_or(ptr::null_mut())
}

/// Builds `tests/native-e2e` with a nested `cargo build` and returns the
/// path of the shared library.
fn build_plugin() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../target"))
        .join("native-e2e");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--quiet"])
        .current_dir(manifest_dir.join("tests/native-e2e"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .status()
        .expect("failed to run cargo");
    assert!(status.success(), "building the native-e2e plugin failed");
    target_dir
        .join("debug")
        .join(format!("{DLL_PREFIX}native_e2e_plugin{DLL_SUFFIX}"))
}

/// The plugin, built and loaded once for all tests, with the world marker
/// checked, the allocator saved, and the import resolver installed.
fn plugin() -> &'static Library {
    static PLUGIN: OnceLock<Library> = OnceLock::new();
    PLUGIN.get_or_init(|| {
        let lib = unsafe { Library::new(build_plugin()) }.expect("failed to dlopen the plugin");

        // Check the world marker first. Its name identifies the world, and
        // calling it returns that name.
        let marker: Symbol<unsafe extern "C" fn() -> *const c_char> =
            unsafe { export(&lib, &format!("__wit_bindgen_world_{WORLD}")) };
        assert_eq!(unsafe { CStr::from_ptr(marker()) }.to_str().unwrap(), WORLD);

        // Both of these are defined by the runtime crate.
        let realloc: Symbol<Realloc> = unsafe { lib.get(b"__wit_bindgen_cabi_realloc\0") }.unwrap();
        REALLOC.store(*realloc as *mut (), Ordering::Release);
        let set_resolver: Symbol<SetImportResolver> =
            unsafe { lib.get(b"__wit_bindgen_set_import_resolver\0") }.unwrap();
        unsafe { set_resolver(Some(resolver), CTX) };

        lib
    })
}

/// Looks up a core export by its unencoded core export name.
unsafe fn export<'a, T>(lib: &'a Library, core_name: &str) -> Symbol<'a, T> {
    let symbol = format!("{}\0", make_external_component(core_name));
    unsafe { lib.get(symbol.as_bytes()) }
        .unwrap_or_else(|e| panic!("plugin does not export `{core_name}`: {e}"))
}

#[test]
fn native_plugin_round_trip() {
    use sync_host::{COUNTERS, LOG};

    let lib = plugin();

    // `greet: func(name: string, times: u32) -> string` covers a string in
    // each direction, a plain import, and a resource constructor, method,
    // and drop.
    let greet: Symbol<unsafe extern "C" fn(*mut u8, usize, i32) -> *mut u8> =
        unsafe { export(lib, "greet") };
    let post_greet: Symbol<unsafe extern "C" fn(*mut u8)> =
        unsafe { export(lib, "cabi_post_greet") };

    let name = "world";
    let arg = unsafe { lower_bytes(name.as_bytes(), 1) };
    let ret = unsafe { greet(arg, name.len(), 3) };
    // The return area holds the string as a `(ptr, len)` pair.
    let out = unsafe {
        let ptr = *(ret as *const *mut u8);
        let len = *(ret.add(size_of::<usize>()) as *const usize);
        String::from_utf8(std::slice::from_raw_parts(ptr, len).to_vec()).unwrap()
    };
    unsafe { post_greet(ret) };

    // The counter starts at 10 and is bumped three times: 11 + 12 + 13.
    assert_eq!(out, "hello world: 36");
    assert_eq!(LOG.lock().unwrap().as_slice(), ["greet(world, 3)"]);
    let counters = COUNTERS.lock().unwrap();
    assert_eq!(counters.created(), 1, "expected exactly one counter");
    assert_eq!(counters.live(), 0, "the counter was never dropped");
    drop(counters);

    // `sum: func(xs: list<u32>) -> u32` covers a non-byte list argument.
    let sum: Symbol<unsafe extern "C" fn(*mut u8, usize) -> i32> = unsafe { export(lib, "sum") };
    let xs: [u32; 4] = [1, 2, 3, 4];
    let bytes = unsafe { std::slice::from_raw_parts(xs.as_ptr().cast::<u8>(), size_of_val(&xs)) };
    let arg = unsafe { lower_bytes(bytes, align_of::<u32>()) };
    assert_eq!(unsafe { sum(arg, xs.len()) }, 10);
}

#[test]
fn native_async_plugin_round_trip() {
    use async_host::{RETURNED, Returned, STREAMS};

    let lib = plugin();

    // An async export takes flattened parameters and returns a callback
    // code. `0` means the task completed, via `task.return`, without needing
    // the `[callback]` export.
    let produce: Symbol<unsafe extern "C" fn(i64) -> i32> =
        unsafe { export(lib, "[async-lift]produce") };

    // Ok path: `fetch(3)` returns `(small, 4)`, the guest writes `1..=4` to
    // the stream and returns `(reader, small, 8)`.
    assert_eq!(unsafe { produce(3) }, 0, "task should exit immediately");
    let returned = RETURNED
        .lock()
        .unwrap()
        .pop()
        .expect("task.return was not called");
    let Returned::Ok {
        reader,
        kind,
        value,
    } = returned
    else {
        panic!("expected the ok arm, got {returned:?}");
    };
    assert_eq!((kind, value), (0, 8));
    let streams = STREAMS.lock().unwrap();
    assert_eq!(streams.created(), 1, "expected exactly one stream");
    assert_eq!(
        reader, 1,
        "reader handle should be the one stream.new handed out"
    );
    let stream = streams.get(reader);
    assert_eq!(stream.buf, [1, 2, 3, 4]);
    assert!(stream.writer_dropped, "the guest should drop its writer");
    drop(streams);

    // Err path: the host lowers the error string into guest memory and it
    // comes back through the err arm of `task.return`.
    assert_eq!(unsafe { produce(-1) }, 0);
    assert_eq!(
        RETURNED.lock().unwrap().pop(),
        Some(Returned::Err("negative id".to_string()))
    );
    assert_eq!(
        STREAMS.lock().unwrap().created(),
        1,
        "no stream on the error path"
    );
}
