use std::cmp::Ordering;
use std::fmt;
use std::ptr;

/// Helper type representing a 1-byte-aligned little-endian value in memory.
///
/// This type is used in slice types for Wasmtime host bindings. Guest types are
/// not guaranteed to be either aligned or in the native endianness. This type
/// wraps these types and provides explicit getters/setters to interact with the
/// underlying value in a safe host-agnostic manner.
#[repr(packed)]
pub struct Le<T>(T);

impl<T> Le<T>
where
    T: Endian,
{
    /// Creates a new `Le<T>` value where the internals are stored in a way
    /// that's safe to copy into wasm linear memory.
    pub fn new(t: T) -> Le<T> {
        Le(t.into_le())
    }

    /// Reads the value stored in this `Le<T>`.
    ///
    /// This will perform a correct read even if the underlying memory is
    /// unaligned, and it will also convert to the host's endianness for the
    /// right representation of `T`.
    pub fn get(&self) -> T {
        unsafe { T::read_unaligned_le(ptr::addr_of!(self.0)) }
    }

    /// Writes the `val` to this slot.
    ///
    /// This will work correctly even if the underlying memory is unaligned and
    /// it will also automatically convert the `val` provided to an endianness
    /// appropriate for WebAssembly (little-endian).
    pub fn set(&mut self, val: T) {
        unsafe { val.write_unaligned_le(ptr::addr_of_mut!(self.0)) }
    }
}

impl<T: Copy> Clone for Le<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> Copy for Le<T> {}

impl<T: Endian + PartialEq> PartialEq for Le<T> {
    fn eq(&self, other: &Le<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Endian + Eq> Eq for Le<T> {}

impl<T: Endian + PartialOrd> PartialOrd for Le<T> {
    fn partial_cmp(&self, other: &Le<T>) -> Option<Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl<T: Endian + Ord> Ord for Le<T> {
    fn cmp(&self, other: &Le<T>) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl<T: Endian + fmt::Debug> fmt::Debug for Le<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: Endian> From<T> for Le<T> {
    fn from(t: T) -> Le<T> {
        Le::new(t)
    }
}

/// Trait used for the implementation of the `Le` type.
pub trait Endian: Copy + Sized {
    /// Converts this value and any aggregate fields (if any) into little-endian
    /// byte order
    fn into_le(self) -> Self;
    /// Reads a value from the provided possibly-unaligned pointer.
    /// Converts from little-endian to the host-endianness.
    unsafe fn read_unaligned_le(ptr: *const Self) -> Self;
    /// Writes a host-value `self` into `ptr`.
    ///
    /// The pointer `ptr` may not be aligned for `Self` and the bytes written
    /// should also be in little-endian order.
    unsafe fn write_unaligned_le(self, ptr: *mut Self);
}

macro_rules! primitives {
    ($($t:ident)*) => ($(
        impl Endian for $t {
            #[inline]
            fn into_le(self) -> Self {
                Self::from_le_bytes(self.to_le_bytes())
            }

            #[inline]
            unsafe fn read_unaligned_le(ptr: *const Self) -> Self {
                Self::from_le_bytes(*ptr.cast())
            }

            #[inline]
            unsafe fn write_unaligned_le(self, ptr: *mut Self) {
                *ptr.cast() = self.to_le_bytes();
            }
        }
    )*)
}

primitives! {
    u16 i16
    u32 i32
    u64 i64
    f32 f64
}
