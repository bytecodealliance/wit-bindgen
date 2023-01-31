use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use wit_bindgen_core::{wit_parser, Files, WorldGenerator};
use wit_parser::{Resolve, UnresolvedPackage};

/// Helper for passing VERSION to opt.
/// If CARGO_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Parser)]
#[command(version = version())]
enum Opt {
    /// This generator outputs a Markdown file describing an interface.
    #[cfg(feature = "markdown")]
    Markdown {
        #[clap(flatten)]
        opts: wit_bindgen_gen_markdown::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for Rust guest modules.
    #[cfg(feature = "rust")]
    Rust {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_rust::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for C/CPP guest modules.
    #[cfg(feature = "c")]
    C {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[clap(flatten)]
        args: Common,
    },

    /// Generates bindings for TeaVM-based Java guest modules.
    #[cfg(feature = "teavm-java")]
    TeavmJava {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_teavm_java::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for TinyGo-based Go guest modules.
    TinyGo {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_go::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: WorldOpt,
    },
}

#[derive(Debug, Parser)]
struct Common {
    /// Where to place output files
    #[clap(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// WIT document to generate bindings for.
    #[clap(value_name = "DOCUMENT", index = 1)]
    wit: PathBuf,

    /// World within the WIT document specified to generate bindings for.
    ///
    /// This can either be `foo` which is the default world in document `foo` or
    /// it's `foo.bar` which is the world named `bar` within document `foo`.
    #[clap(short, long)]
    world: Option<String>,
}

fn main() -> Result<()> {
    let mut files = Files::default();
    let (generator, opt) = match Opt::parse() {
        #[cfg(feature = "markdown")]
        Opt::Markdown { opts, args } => (opts.build(), args),
        #[cfg(feature = "c")]
        Opt::C { opts, args } => (opts.build(), args),
        #[cfg(feature = "rust")]
        Opt::Rust { opts, args } => (opts.build(), args),
        #[cfg(feature = "teavm-java")]
        Opt::TeavmJava { opts, args } => (opts.build(), args),
        #[cfg(feature = "go")]
        Opt::TeavmJava { opts, args } => (opts.build(), args),
    };

    gen_world(generator, &opt, &mut files)?;

    for (name, contents) in files.iter() {
    Opt::command().debug_assert()
}
