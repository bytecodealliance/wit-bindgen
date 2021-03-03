use std::alloc::{self, Layout};
pub use witx_bindgen_rust_impl::{export, import};

#[no_mangle]
unsafe extern "C" fn witx_malloc(len: usize) -> *mut u8 {
    let layout = Layout::from_size_align_unchecked(len, 8);
    let ptr = alloc::alloc(layout);
    if ptr.is_null() {
        alloc::handle_alloc_error(layout);
    }
    return ptr;
}

#[no_mangle]
unsafe extern "C" fn witx_free(ptr: *mut u8, len: usize) {
    let layout = Layout::from_size_align_unchecked(len, 8);
    alloc::dealloc(ptr, layout);
}
