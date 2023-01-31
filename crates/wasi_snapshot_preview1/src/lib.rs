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

#![allow(unused_variables)]

use std::arch::wasm32::unreachable;
use wasi::*;

wit_bindgen_guest_rust::generate!("testwasi");

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
    if fd != 1 && fd != 2 {
        unreachable();
    }
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

        let slice = core::slice::from_raw_parts(ptr, len);
        if fd == 1 {
            testwasi::log(slice);
        } else {
            testwasi::log_err(slice);
        }

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

#[no_mangle]
pub extern "C" fn fd_fdstat_get(fd: Fd, fdstat: *mut Fdstat) -> Errno {
    if fd != 1 {
        unreachable();
    }

    unsafe {
        (*fdstat).fs_filetype = FILETYPE_UNKNOWN;
        (*fdstat).fs_flags = FDFLAGS_APPEND;
        (*fdstat).fs_rights_base = RIGHTS_FD_WRITE;
        (*fdstat).fs_rights_inheriting = RIGHTS_FD_WRITE;
    }

    ERRNO_SUCCESS
}
