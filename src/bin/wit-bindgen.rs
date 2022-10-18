use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use wit_bindgen_core::component::ComponentGenerator;
use wit_bindgen_core::{wit_parser, Files, Generator, WorldGenerator};
use wit_component::ComponentInterfaces;
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
        world: LegacyWorld,
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
        world: LegacyWorld,
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
        world: World,
    },
    /// Generates bindings for C/CPP guest modules.
    C {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: LegacyWorld,
    },
    /// Generates bindings for TeaVM-based Java guest modules.
    TeavmJava {
        #[clap(flatten)]
        opts: wit_bindgen_gen_guest_teavm_java::Opts,
        #[clap(flatten)]
        common: Common,
        #[clap(flatten)]
        world: LegacyWorld,
    },
}

#[derive(Debug, Parser)]
struct LegacyWorld {
    /// Generate import bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long, short)]
    imports: Vec<PathBuf>,

    /// Generate export bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[clap(long, short)]
    exports: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
struct World {
    /// Generate bindings for the guest import interfaces specified.
    #[clap(long = "import", short, value_name = "[NAME=]INTERFACE", value_parser = parse_named_interface)]
    imports: Vec<Interface>,

    /// Generate bindings for the guest export interfaces specified.
    #[clap(long = "export", short, value_name = "[NAME=]INTERFACE", value_parser = parse_named_interface)]
    exports: Vec<Interface>,

    /// Generate bindings for the guest default export interface specified.
    #[clap(long, short, value_name = "[NAME=]INTERFACE", value_parser = parse_named_interface)]
    default: Option<Interface>,

    /// The top-level name of the generated bindings, which may be used for
    /// naming modules/files/etc.
    #[clap(long, short)]
    name: String,
}

fn parse_named_interface(s: &str) -> Result<Interface> {
    let mut parts = s.splitn(2, '=');
    let name_or_path = parts.next().unwrap();
    let (name, path) = match parts.next() {
        Some(path) => (name_or_path, path),
        None => {
            let name = Path::new(name_or_path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap();
            (name, name_or_path)
        }
    };
    let path = Path::new(path);
    if !path.is_file() {
        bail!("interface file `{}` does not exist", path.display());
    }

    let mut interface = Interface::parse_file(&path)
        .with_context(|| format!("failed to parse interface file `{}`", path.display()))?;

    interface.name = name.to_string();

    Ok(interface)
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
            | Category::Host(HostGenerator::WasmtimePy { common, .. })
            | Category::Markdown { common, .. } => common,
            Category::Host(HostGenerator::Js { component, .. }) => &component.common,
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
        Category::Host(HostGenerator::WasmtimePy { opts, world, .. }) => {
            gen_legacy_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Host(HostGenerator::Js { opts, component }) => {
            gen_component(opts.build(), component, &mut files)?;
        }
        Category::Guest(GuestGenerator::Rust { opts, world, .. }) => {
            gen_world(opts.build(), world, &mut files)?;
        }
        Category::Guest(GuestGenerator::C { opts, world, .. }) => {
            gen_legacy_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Guest(GuestGenerator::TeavmJava { opts, world, .. }) => {
            gen_legacy_world(Box::new(opts.build()), world, &mut files)?;
        }
        Category::Markdown { opts, world, .. } => {
            gen_legacy_world(Box::new(opts.build()), world, &mut files)?;
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

fn gen_legacy_world(
    mut generator: Box<dyn Generator>,
    world: LegacyWorld,
    files: &mut Files,
) -> Result<()> {
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

fn gen_world(
    mut generator: Box<dyn WorldGenerator>,
    world: World,
    files: &mut Files,
) -> Result<()> {
    let imports = world
        .imports
        .into_iter()
        .map(|i| (i.name.clone(), i))
        .collect();
    let exports = world
        .exports
        .into_iter()
        .map(|i| (i.name.clone(), i))
        .collect();
    let interfaces = ComponentInterfaces {
        imports,
        exports,
        default: world.default,
    };
    generator.generate(&world.name, &interfaces, files);
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
