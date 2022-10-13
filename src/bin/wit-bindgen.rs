use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use wit_bindgen_core::{wit_parser, Files, Generator};
use wit_parser::Interface;

/// Helper for passing VERSION to opt.
/// If CARGO_VERSION_INFO is set, use it, otherwise use CARGO_PKG_VERSION.
fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
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
        #[clap(flatten)]
        world: World,
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
        #[clap(flatten)]
        world: World,
    },
    /// Generates bindings for Python hosts using the Wasmtime engine.
    WasmtimePy {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_py::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: World,
    },
    /// Generates bindings for JavaScript hosts.
    Js {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_js::Opts,

        component: PathBuf,
        #[clap(flatten)]
        common: Common,

        #[clap(long)]
        name: Option<String>,
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
        #[clap(flatten)]
        world: World,
    },
    /// Generates bindings for C/CPP guest modules.
    C {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: World,
    },
    /// Generates bindings for TeaVM-based Java guest modules.
    TeavmJava {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_teavm_java::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: World,
    },
}

#[derive(Debug, Parser)]
struct World {
    /// Generate import bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long = "import", short)]
    imports: Vec<PathBuf>,

    /// Generate export bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long = "export", short)]
    exports: Vec<PathBuf>,
}

#[derive(Debug, Parser, Clone)]
struct Common {
    /// Where to place output files
    #[clap(long = "out-dir")]
    out_dir: Option<PathBuf>,
}

impl Opt {
    fn common(&self) -> &Common {
        match &self.category {
            Category::Guest(GuestGenerator::Rust { common, .. })
            | Category::Guest(GuestGenerator::C { common, .. })
            | Category::Guest(GuestGenerator::TeavmJava { common, .. })
            | Category::Host(HostGenerator::WasmtimeRust { common, .. })
            | Category::Host(HostGenerator::WasmtimePy { common, .. })
            | Category::Host(HostGenerator::Js { common, .. })
            | Category::Markdown { common, .. } => common,
        }
    }
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();
    let common = opt.common().clone();

    let mut files = Files::default();
    match opt.category {
        Category::Guest(GuestGenerator::Rust { opts, world, .. }) => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Host(HostGenerator::WasmtimeRust { opts, world, .. }) => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Host(HostGenerator::WasmtimePy { opts, world, .. }) => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Host(HostGenerator::Js {
            opts,
            component,
            name,
            ..
        }) => {
            let wasm = wat::parse_file(&component)?;
            let name = match &name {
                Some(name) => name.as_str(),
                None => component
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| anyhow!("filename not valid utf-8"))?,
            };
            opts.generate(name, &wasm, &mut files)?;
        }
        Category::Guest(GuestGenerator::C { opts, world, .. }) => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Guest(GuestGenerator::TeavmJava { opts, world, .. }) => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Markdown { opts, world, .. } => {
            gen_world(Box::new(opts.build()), world, &mut files)?;
        }
    }

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

fn gen_world(mut generator: Box<dyn Generator>, world: World, files: &mut Files) -> Result<()> {
    let imports = world
        .imports
        .iter()
        .map(|wit| Interface::parse_file(wit))
        .collect::<Result<Vec<_>>>()?;
    let exports = world
        .exports
        .iter()
        .map(|wit| Interface::parse_file(wit))
        .collect::<Result<Vec<_>>>()?;

    generator.generate_all(&imports, &exports, files);
    Ok(())
}
