include!(env!("BINDINGS"));

use crate::exports::test::arena_allocated_resources::to_test::{Guest, GuestThing, ThingStorage};

export!(Component);

struct Component;

impl Guest for Component {
    type Thing = MyThing;
}

mod arena {

    use core::sync::atomic::{AtomicUsize, Ordering};
    use core::{cell::UnsafeCell, mem::MaybeUninit};

    /// A simple no_std arena allocator for fixed-size allocations.
    ///
    /// The arena allocates items of type T sequentially from a pre-allocated buffer
    /// and does not support individual deallocation. Memory is reclaimed
    /// only when the entire arena is reset.
    pub struct Arena<T, const SIZE: usize> {
        buffer: [UnsafeCell<MaybeUninit<T>>; SIZE],
        offset: AtomicUsize,
    }

    // Element allocation is atomic and elements are exclusively handed out after allocation,
    // so the arena can be send to other threads and simultaneosly accessed by multiple threads
    unsafe impl<T: Sync, const SIZE: usize> Sync for Arena<T, SIZE> {}
    unsafe impl<T: Send, const SIZE: usize> Send for Arena<T, SIZE> {}

    impl<T: Default, const SIZE: usize> Arena<T, SIZE> {
        pub const fn new() -> Self {
            Self {
                buffer: [const { UnsafeCell::new(MaybeUninit::uninit()) }; SIZE],
                offset: AtomicUsize::new(0),
            }
        }

        /// Allocates space for a single item of type T.
        /// Returns a mutable reference to the allocated memory, or None if there's insufficient space.
        pub fn alloc_one(&self) -> Option<&mut T> {
            // short circuit the exhausted state (don't increment if full)
            if self.offset.load(Ordering::Relaxed) >= SIZE {
                None
            } else {
                // now try to allocate for real
                let pos = self.offset.fetch_add(1, Ordering::Acquire);
                if pos >= SIZE {
                    // now self.offset is already beyond SIZE, reduce our increment and return none
                    self.offset.fetch_sub(1, Ordering::Release);
                    None
                } else {
                    let ptr = self.buffer[pos].get();
                    // SAFETY: we demand exclusive ownership of the item in the arena
                    let uninit = unsafe { &mut *ptr };
                    Some(uninit.write(Default::default()))
                }
            }
        }
    }
}

use arena::Arena;

#[derive(Clone)]
struct MyThing {
    contents: u32,
}

static ARENA: Arena<Option<MyThing>, 4> = Arena::new();

impl GuestThing for MyThing {
    fn new(v: u32) -> MyThing {
        MyThing { contents: v }
    }

    fn get(&self) -> u32 {
        self.contents
    }

    fn resource_into_raw_(val: ThingStorage<Self>) -> *mut ThingStorage<Self>
    where
        Self: Sized,
    {
        val.and_then(|v| {
            ARENA.alloc_one().map(|x| {
                *x = Some(v);
                x as *mut _
            })
        })
        .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn resource_from_raw_(handle: *mut ThingStorage<Self>) -> ThingStorage<Self>
    where
        Self: Sized,
    {
        let res = unsafe { &mut *handle }.take();
        res
    }
}
