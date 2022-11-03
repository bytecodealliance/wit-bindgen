use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::{wit_parser, Files, WorldGenerator};
use wit_component::ComponentInterfaces;
use wit_parser::World;

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
        world: WorldOpt,
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
        world: WorldOpt,
    },
    /// Generates bindings for Python hosts using the Wasmtime engine.
    WasmtimePy {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_py::Opts,
        #[clap(flatten)]
        component: ComponentOpts,
    },
    /// Generates bindings for JavaScript hosts.
    Js {
        #[clap(flatten)]
        opts: wit_bindgen_gen_host_js::Opts,
        #[clap(flatten)]
        component: ComponentOpts,
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
        world: WorldOpt,
    },
    /// Generates bindings for C/CPP guest modules.
    C {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: WorldOpt,
    },
    /// Generates bindings for TeaVM-based Java guest modules.
    TeavmJava {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_teavm_java::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: WorldOpt,
    },
}

#[derive(Debug, Parser)]
struct WorldOpt {
    /// The top-level name of the generated bindings, which may be used for
    /// naming modules/files/etc.
    #[clap(long, short)]
    name: Option<String>,

    /// Generate bindings for the WIT document.
    #[clap(value_name = "DOCUMENT", value_parser = parse_world)]
    wit: World,
}

fn parse_world(s: &str) -> Result<World> {
    let path = Path::new(s);
    if !path.is_file() {
        bail!("wit file `{}` does not exist", path.display());
    }

    let world = World::parse_file(&path)
        .with_context(|| format!("failed to parse wit file `{}`", path.display()))
        .map_err(|e| {
            eprintln!("{e:?}");
            e
        })?;

    Ok(world)
}

#[derive(Debug, Parser)]
struct ComponentOpts {
    /// Path to the input wasm component to generate bindings for.
    component: PathBuf,

    /// Optionally rename the generated bindings instead of inferring the name
    /// from the input `component` path.
    #[clap(long)]
    name: Option<String>,

    #[clap(flatten)]
    common: Common,
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
            | Category::Markdown { common, .. } => common,
            Category::Host(HostGenerator::Js { component, .. })
            | Category::Host(HostGenerator::WasmtimePy { component, .. }) => &component.common,
        }
    }
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();
    let common = opt.common().clone();

    let mut files = Files::default();
    match opt.category {
        Category::Host(HostGenerator::WasmtimeRust { opts, world, .. }) => {
            gen_world(opts.build(), world, &mut files)?;
        }
        Category::Host(HostGenerator::WasmtimePy { opts, component }) => {
            gen_component(opts.build(), component, &mut files)?;
        }
        Category::Host(HostGenerator::Js { opts, component }) => {
            gen_component(opts.build()?, component, &mut files)?;
        }
        Category::Guest(GuestGenerator::Rust { opts, world, .. }) => {
            gen_world(opts.build(), world, &mut files)?;
        }
        Category::Guest(GuestGenerator::C { opts, world, .. }) => {
            gen_world(opts.build(), world, &mut files)?;
        }
        Category::Guest(GuestGenerator::TeavmJava { opts, world, .. }) => {
            gen_world(opts.build(), world, &mut files)?;
        }
        Category::Markdown { opts, world, .. } => {
            gen_world(opts.build(), world, &mut files)?;
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

fn gen_world(
    mut generator: Box<dyn WorldGenerator>,
    opts: WorldOpt,
    files: &mut Files,
) -> Result<()> {
    let World {
        name,
        imports,
        exports,
        default,
    } = opts.wit;

    let interfaces = ComponentInterfaces {
        exports,
        imports,
        default,
    };

    let name = match opts.name {
        Some(name) => name,
        None => name,
    };
    generator.generate(&name, &interfaces, files);
    Ok(())
}

fn gen_component(
    mut generator: Box<dyn ComponentGenerator>,
    opts: ComponentOpts,
    files: &mut Files,
) -> Result<()> {
    let wasm = wat::parse_file(&opts.component)?;
    let name = match &opts.name {
        Some(name) => name.as_str(),
        None => opts
            .component
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("filename not valid utf-8"))?,
    };

    wit_bindgen_core::component::generate(&mut *generator, name, &wasm, files)?;

    Ok(())
}
