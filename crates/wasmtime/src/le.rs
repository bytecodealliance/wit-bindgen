use crate::AllBytesValid;
use std::cmp::Ordering;
use std::fmt;

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
        self.0.from_le()
    }

    /// Writes the `val` to this slot.
    ///
    /// This will work correctly even if the underlying memory is unaligned and
    /// it will also automatically convert the `val` provided to an endianness
    /// appropriate for WebAssembly (little-endian).
    pub fn set(&mut self, val: T) {
        self.0 = val.into_le();
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

unsafe impl<T: AllBytesValid> AllBytesValid for Le<T> {}

/// Trait used for the implementation of the `Le` type.
pub trait Endian: Copy + Sized {
    /// Converts this value and any aggregate fields (if any) into little-endian
    /// byte order
    fn into_le(self) -> Self;
    /// Converts this value and any aggregate fields (if any) from
    /// little-endian byte order
    fn from_le(self) -> Self;
}

macro_rules! primitives {
    ($($t:ident)*) => ($(
        impl Endian for $t {
            #[inline]
            fn into_le(self) -> Self {
                Self::from_ne_bytes(self.to_le_bytes())
            }

            #[inline]
            fn from_le(self) -> Self {
                Self::from_le_bytes(self.to_ne_bytes())
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

macro_rules! tuples {
    ($(($($t:ident)*))*) => ($(
        #[allow(non_snake_case)]
        impl <$($t:Endian,)*> Endian for ($($t,)*) {
            fn into_le(self) -> Self {
                let ($($t,)*) = self;
                ($($t.into_le(),)*)
            }

            fn from_le(self) -> Self {
                let ($($t,)*) = self;
                ($($t.from_le(),)*)
            }
        }
    )*)
}

tuples! {
    ()
    (T1)
    (T1 T2)
    (T1 T2 T3)
    (T1 T2 T3 T4)
    (T1 T2 T3 T4 T5)
    (T1 T2 T3 T4 T5 T6)
    (T1 T2 T3 T4 T5 T6 T7)
    (T1 T2 T3 T4 T5 T6 T7 T8)
    (T1 T2 T3 T4 T5 T6 T7 T8 T9)
    (T1 T2 T3 T4 T5 T6 T7 T8 T9 T10)
}
