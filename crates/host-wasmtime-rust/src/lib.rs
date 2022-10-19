pub use wit_bindgen_host_wasmtime_rust_macro::*;

#[cfg(feature = "tracing-lib")]
pub use tracing_lib as tracing;
#[doc(hidden)]
pub use {anyhow, async_trait::async_trait, wasmtime};
