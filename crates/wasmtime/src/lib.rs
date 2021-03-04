pub use witx_bindgen_wasmtime_impl::{export, import};

#[doc(hidden)]
pub use {anyhow, bitflags, wasmtime};

mod error;
mod memory;
mod ptr;
mod region;
mod table;

pub use error::GuestError;
pub use memory::{BorrowHandle, GuestMemory};
pub use ptr::{GuestPtr, Pointee};
pub use region::{BorrowChecker, Region};
pub use table::*;

pub struct WasmtimeGuestMemory<'a> {
    mem: &'a wasmtime::Memory,
    borrows: &'a BorrowChecker,
}

impl<'a> WasmtimeGuestMemory<'a> {
    // Unsafe because we don't know that `borrows` are valid for `mem
    pub unsafe fn new(
        mem: &'a wasmtime::Memory,
        borrows: &'a BorrowChecker,
    ) -> WasmtimeGuestMemory<'a> {
        WasmtimeGuestMemory { mem, borrows }
    }
}

unsafe impl<'a> GuestMemory for WasmtimeGuestMemory<'a> {
    fn base(&self) -> (*mut u8, u32) {
        (self.mem.data_ptr(), self.mem.data_size() as u32)
    }
    fn has_outstanding_borrows(&self) -> bool {
        self.borrows.has_outstanding_borrows()
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        self.borrows.is_mut_borrowed(r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        self.borrows.is_shared_borrowed(r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.borrows.mut_borrow(r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.borrows.shared_borrow(r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        self.borrows.mut_unborrow(h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        self.borrows.shared_unborrow(h)
    }
}

// /// A runtime-independent way for Wiggle to terminate WebAssembly execution.
// /// Functions that are marked `(@witx noreturn)` will always return a Trap.
// /// Other functions that want to Trap can do so via their `UserErrorConversion`
// /// trait, which transforms the user's own error type into a `Result<abierror, Trap>`.
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum Trap {
//     /// A Trap which indicates an i32 (posix-style) exit code. Runtimes may have a
//     /// special way of dealing with this for WASI embeddings and otherwise.
//     I32Exit(i32),
//     /// Any other Trap is just an unstructured String, for reporting and debugging.
//     String(String),
// }

// impl From<GuestError> for Trap {
//     fn from(err: GuestError) -> Trap {
//         Trap::String(err.to_string())
//     }
// }
