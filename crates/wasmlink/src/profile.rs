use wasmparser::FuncType;

/// Represents a link profile.
///
/// Link profiles represent information about the target environment.
#[derive(Debug)]
pub struct Profile {}

impl Profile {
    /// Constructs a new link profile.
    pub fn new() -> Self {
        Self {}
    }

    /// Determines if the profile provides the given import.
    pub fn provides(&self, module: &str, _field: Option<&str>, _ty: &FuncType) -> bool {
        // TODO: provide some actual implementation for this
        module == "wasi_snapshot_preview1"
    }
}
