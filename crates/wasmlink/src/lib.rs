//! WebAssembly linker API.

#![deny(missing_docs)]

mod adapted;
mod linker;
mod module;
mod profile;
mod resources;

pub use self::linker::Linker;
pub use self::module::Module;
pub use self::profile::Profile;
