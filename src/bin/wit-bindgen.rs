use anyhow::{Context, Result};
use clap::Parser;
use lazy_static::lazy_static;
use std::path::PathBuf;
use wit_bindgen_core::{wit_parser, Files, Generator};
use wit_parser::Interface;

lazy_static! {
    pub static ref VERSION: String = build_info();
}

/// Helper for passing VERSION to opt.
fn version() -> &'static str {
    &VERSION
}

#[derive(Debug, Parser)]
#[command(version = version())]
struct Opt {
    #[command(subcommand)]
    category: Category,
}

#[derive(Debug, Parser)]
enum Category {
    /// Generators for creating hosts that embed WASM modules/components.
    #[command(subcommand)]
    Host(HostGenerator),
    /// Generators for writing guest WASM modules/components.
    #[command(subcommand)]
    Guest(GuestGenerator),
    /// This generator outputs a Markdown file describing an interface.
    Markdown {
        #[clap(flatten)]
        opts: wit_bindgen_gen_markdown::Opts,
        #[clap(flatten)]
        common: Common,
    },
}

#[derive(Debug, Parser)]
enum HostGenerator {
    /// Generates bindings for Rust hosts using the Wasmtime engine.
    WasmtimeRust {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_rust::Opts,
        #[clap(flatten)]
        common: Common,
    },
    /// Generates bindings for Python hosts using the Wasmtime engine.
    WasmtimePy {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_py::Opts,
        #[clap(flatten)]
        common: Common,
    },
    /// Generates bindings for JavaScript hosts.
    Js {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_js::Opts,
        #[clap(flatten)]
        common: Common,
    },
}

#[derive(Debug, Parser)]
enum GuestGenerator {
    /// Generates bindings for Rust guest modules.
    Rust {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_rust::Opts,
        #[clap(flatten)]
        common: Common,
    },
    /// Generates bindings for C/CPP guest modules.
    C {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[clap(flatten)]
        common: Common,
    },
    /// Generates bindings for TeaVM-based Java guest modules.
    TeavmJava {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_teavm_java::Opts,
        #[clap(flatten)]
        common: Common,
    },
}

#[derive(Debug, Parser)]
struct Common {
    /// Where to place output files
    #[clap(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Generate import bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long = "import", short)]
    imports: Vec<PathBuf>,

    /// Generate export bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long = "export", short)]
    exports: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();
    let (mut generator, common): (Box<dyn Generator>, _) = match opt.category {
        Category::Guest(GuestGenerator::Rust { opts, common }) => (Box::new(opts.build()), common),
        Category::Host(HostGenerator::WasmtimeRust { opts, common }) => {
            (Box::new(opts.build()), common)
        }
        Category::Host(HostGenerator::WasmtimePy { opts, common }) => {
            (Box::new(opts.build()), common)
        }
        Category::Host(HostGenerator::Js { opts, common }) => (Box::new(opts.build()), common),
        Category::Guest(GuestGenerator::C { opts, common }) => (Box::new(opts.build()), common),
        Category::Guest(GuestGenerator::TeavmJava { opts, common }) => {
            (Box::new(opts.build()), common)
        }
        Category::Markdown { opts, common } => (Box::new(opts.build()), common),
    };

    let imports = common
        .imports
        .iter()
        .map(|wit| Interface::parse_file(wit))
        .collect::<Result<Vec<_>>>()?;
    let exports = common
        .exports
        .iter()
        .map(|wit| Interface::parse_file(wit))
        .collect::<Result<Vec<_>>>()?;

    let mut files = Files::default();
    generator.generate_all(&imports, &exports, &mut files);

    for (name, contents) in files.iter() {
        let dst = match &common.out_dir {
            Some(path) => path.join(name),
            None => name.into(),
        };
        println!("Generating {:?}", dst);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {:?}", parent))?;
        }
        std::fs::write(&dst, contents).with_context(|| format!("failed to write {:?}", dst))?;
    }

    Ok(())
}

/// Returns build information, similar to: 0.1.0 (2be4034 2022-03-31).
fn build_info() -> String {
    format!(
        "{} ({} {})",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_COMMIT_SHORT_HASH"),
        env!("CARGO_COMMIT_DATE")
    )
}
