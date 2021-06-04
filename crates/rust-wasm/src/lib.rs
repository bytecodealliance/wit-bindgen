use std::alloc::{self, Layout};
pub use witx_bindgen_rust_impl::{export, import};

#[no_mangle]
unsafe extern "C" fn canonical_abi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    len: usize,
    align: usize,
) -> *mut u8 {
    let layout;
    let ptr = if old_len == 0 {
        layout = Layout::from_size_align_unchecked(len, align);
        alloc::alloc(layout)
    } else {
        layout = Layout::from_size_align_unchecked(old_len, align);
        alloc::realloc(old_ptr, layout, len)
    };
    if ptr.is_null() {
        alloc::handle_alloc_error(layout);
    }
    return ptr;
}

#[no_mangle]
unsafe extern "C" fn canonical_abi_free(ptr: *mut u8, len: usize, align: usize) {
    let layout = Layout::from_size_align_unchecked(len, align);
    alloc::dealloc(ptr, layout);
}

pub mod exports;
pub mod imports;
