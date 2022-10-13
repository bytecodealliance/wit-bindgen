//! This module is intended to be an "adapter module" fed into `wit-component`
//! to translate the `wasi_snapshot_preview1` ABI into an ABI that uses the
//! component model. This library is compiled as a standalone wasm file and is
//! used to implement `wasi_snapshot_preview1` interfaces required by the tests
//! throughout the `wit-bindgen` repository.
//!
//! This is not intended to be a comprehensive polyfill. Instead this is just
//! the bare bones necessary to get `wit-bindgen` itself and its tests working.
//!
//! Currently all functions are trapping stubs since nothing actually runs the
//! output component just yet. These stubs should get filled in as necessary
//! once hosts start running components. The current assumption is that the
//! imports will be adapted to a custom `wit-bindgen`-specific host `*.wit` file
//! which is only suitable for `wit-bindgen` tests.

#![no_std]
#![allow(unused_variables)]

use core::arch::wasm32::unreachable;
use wasi::*;

wit_bindgen_guest_rust::import!({ paths: ["testwasi.wit"], no_std });

// Nothing in this wasm module should end up needing cabi_realloc. However, if
// we don't define this trapping implementation of the export, we'll pull in
// the one from wit_bindgen_guest_rust, which will pull in the libc allocator
// and a bunch of panic related machinery from std.
#[no_mangle]
unsafe extern "C" fn cabi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    align: usize,
    new_len: usize,
) -> *mut u8 {
    unreachable()
}

#[no_mangle]
pub extern "C" fn environ_get(environ: *mut *mut u8, environ_buf: *mut u8) -> Errno {
    ERRNO_SUCCESS
}

#[no_mangle]
pub extern "C" fn environ_sizes_get(environc: *mut Size, environ_buf_size: *mut Size) -> Errno {
    unsafe {
        *environc = 0;
        *environ_buf_size = 0;
    }
    ERRNO_SUCCESS
}

#[no_mangle]
pub extern "C" fn args_get(args: *mut *mut u8, args_buf: *mut u8) -> Errno {
    ERRNO_SUCCESS
}

#[no_mangle]
pub extern "C" fn args_sizes_get(argc: *mut Size, arg_buf_size: *mut Size) -> Errno {
    unsafe {
        *argc = 0;
        *arg_buf_size = 0;
    }
    ERRNO_SUCCESS
}

#[no_mangle]
pub extern "C" fn clock_time_get(
    clockid: Clockid,
    precision: Timestamp,
    out: *mut Timestamp,
) -> Errno {
    unreachable()
}

#[no_mangle]
pub extern "C" fn fd_write(
    fd: Fd,
    mut iovs_ptr: *const Ciovec,
    mut iovs_len: usize,
    nwritten: *mut Size,
) -> Errno {
    unsafe {
        // Advance to the first non-empty buffer.
        while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
            iovs_ptr = iovs_ptr.add(1);
            iovs_len -= 1;
        }
        if iovs_len == 0 {
            *nwritten = 0;
            return ERRNO_SUCCESS;
        }

        let ptr = (*iovs_ptr).buf;
        let len = (*iovs_ptr).buf_len;

        testwasi::log(core::slice::from_raw_parts(ptr, len));

        *nwritten = len;
    }
    ERRNO_SUCCESS
}

#[no_mangle]
pub extern "C" fn fd_seek(fd: Fd, offset: Filedelta, whence: Whence, filesize: *mut Size) -> Errno {
    unreachable()
}

#[no_mangle]
pub extern "C" fn fd_close(fd: Fd) -> Errno {
    unreachable()
}

#[no_mangle]
pub extern "C" fn proc_exit(rval: Exitcode) -> ! {
    unreachable()
}
