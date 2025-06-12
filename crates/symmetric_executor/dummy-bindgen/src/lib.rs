// this crate tries to minimize dependencies for symmetric bindings

#[cfg(feature = "symmetric")]
pub mod rt {
    pub use dummy_rt::rt::maybe_link_cabi_realloc;
    pub use wit_bindgen_symmetric_rt::{async_support, run, Cleanup, EventGenerator, EventSubscription};
}

#[cfg(feature = "canonical")]
pub use original::rt;
