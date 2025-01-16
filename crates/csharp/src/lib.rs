use wit_bindgen_core::WorldGenerator;
use wit_component::StringEncoding;

mod csharp_ident;
mod csproj;
mod function;
mod interface;
mod world_generator;

pub use csproj::CSProject;

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    #[cfg_attr(feature = "clap", arg(long, default_value_t = StringEncoding::default()))]
    pub string_encoding: StringEncoding,

    /// Whether or not to generate a stub class for exported functions
    #[cfg_attr(feature = "clap", arg(long))]
    pub generate_stub: bool,

    // TODO: This should only temporarily needed until mono and native aot aligns.
    #[cfg_attr(feature = "clap", arg(short, long, value_enum))]
    pub runtime: CSharpRuntime,

    /// Use the `internal` access modifier by default instead of `public`
    #[cfg_attr(feature = "clap", arg(long))]
    pub internal: bool,

    /// Skip generating `cabi_realloc`, `WasmImportLinkageAttribute`, and component type files
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip_support_files: bool,

    /// Generate code for WIT `Result` types instead of exceptions
    #[cfg_attr(feature = "clap", arg(long))]
    pub with_wit_results: bool,
}

impl Opts {
    pub fn build(&self) -> Box<dyn WorldGenerator> {
        Box::new(world_generator::CSharp {
            opts: self.clone(),
            ..world_generator::CSharp::default()
        })
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum CSharpRuntime {
    #[default]
    NativeAOT,
    Mono,
}
