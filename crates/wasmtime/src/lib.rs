pub use witx_bindgen_wasmtime_impl::{export, import};

#[doc(hidden)]
pub use {anyhow, bitflags, wasmtime};

mod error;
pub mod exports;
pub mod imports;
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
