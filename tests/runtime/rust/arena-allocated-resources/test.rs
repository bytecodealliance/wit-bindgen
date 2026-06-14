include!(env!("BINDINGS"));

use crate::exports::test::arena_allocated_resources::to_test::{Guest, GuestThing};

export!(Component);

struct Component;

impl Guest for Component {
    type Thing = MyThing;
}

mod arena {

    use core::{cell::UnsafeCell, mem::MaybeUninit};

    /// A simple no_std arena allocator for fixed-size allocations.
    ///
    /// The arena allocates items of type T sequentially from a pre-allocated buffer
    /// and does not support individual deallocation. Memory is reclaimed
    /// only when the entire arena is reset.
    pub struct Arena<T, const SIZE: usize> {
        buffer: [MaybeUninit<T>; SIZE],
        offset: usize,
    }

    impl<T, const SIZE: usize> Arena<T, SIZE> {
        /// Allocates space for a single item of type T.
        /// Returns a mutable reference to the allocated memory, or None if there's insufficient space.
        pub fn alloc_one(&mut self) -> Option<&mut T> {
            if self.offset < SIZE {
                let ptr = self.buffer[self.offset].as_mut_ptr();
                self.offset += 1;
                Some(unsafe { &mut *ptr })
            } else {
                None
            }
        }
    }

    /// A static-safe wrapper for Arena that uses interior mutability.
    ///
    /// This allows an Arena to be stored in a static variable and accessed safely
    /// in single-threaded contexts without requiring std or alloc.
    ///
    /// # Safety
    ///
    /// This type is safe to use in single-threaded environments. In multi-threaded
    /// contexts, external synchronization is required.
    pub struct StaticArena<T, const SIZE: usize> {
        arena: UnsafeCell<Arena<T, SIZE>>,
    }

    // SAFETY: StaticArena is Sync because we enforce single-threaded access through
    // the API. It can be safely shared across threads as long as only one thread
    // accesses it at a time (which is the responsibility of the user).
    unsafe impl<T, const SIZE: usize> Sync for StaticArena<T, SIZE> where T: Sync {}

    // SAFETY: StaticArena is Send because the underlying Arena can be moved between
    // threads, and T itself must be Send.
    unsafe impl<T, const SIZE: usize> Send for StaticArena<T, SIZE> where T: Send {}

    impl<T, const SIZE: usize> StaticArena<T, SIZE> {
        /// Creates a new static arena.
        pub const fn new() -> Self {
            StaticArena {
                arena: UnsafeCell::new(Arena {
                    buffer: [const { MaybeUninit::uninit() }; SIZE],
                    offset: 0,
                }),
            }
        }

        /// Gets mutable access to the arena.
        ///
        /// # Safety
        ///
        /// This is safe in single-threaded contexts. In multi-threaded contexts,
        /// the caller must ensure exclusive access.
        #[inline]
        pub fn get_mut(&self) -> &mut Arena<T, SIZE> {
            unsafe { &mut *self.arena.get() }
        }

        /// Allocates a single item.
        pub fn alloc_one(&self) -> Option<&mut T> {
            self.get_mut().alloc_one()
        }
    }
}

use arena::StaticArena;

#[derive(Clone)]
struct MyThing {
    contents: u32,
}

static ARENA: StaticArena<Option<MyThing>, 4> = StaticArena::new();

impl GuestThing for MyThing {
    fn new(v: u32) -> MyThing {
        MyThing { contents: v }
    }
    fn get(&self) -> u32 {
        self.contents
    }
    fn _resource_into_raw(val: Option<Self>) -> *mut Option<Self>
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
    unsafe fn _resource_from_raw(handle: *mut Option<Self>) -> Option<Self>
    where
        Self: Sized,
    {
        let res = unsafe { &mut *handle }.take();
        res
    }
}
