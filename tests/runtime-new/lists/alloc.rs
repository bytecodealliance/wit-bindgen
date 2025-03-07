use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

#[global_allocator]
static ALLOC: A = A;

static ALLOC_AMT: AtomicUsize = AtomicUsize::new(0);

struct A;

unsafe impl GlobalAlloc for A {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOC_AMT.fetch_add(layout.size(), SeqCst);
        }
        return ptr;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Poison all deallocations to try to catch any use-after-free in the
        // bindings as early as possible.
        std::ptr::write_bytes(ptr, 0xde, layout.size());
        ALLOC_AMT.fetch_sub(layout.size(), SeqCst);
        System.dealloc(ptr, layout)
    }
}
pub fn get() -> usize {
    ALLOC_AMT.load(SeqCst)
}
