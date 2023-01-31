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
struct Opt {
    #[command(subcommand)]
    category: Category,
    #[clap(flatten)]
    common: Common,
    #[clap(flatten)]
    world: WorldOpt,
}

#[derive(Debug, Parser)]
enum Category {
    /// This generator outputs a Markdown file describing an interface.
    #[cfg(feature = "markdown")]
    Markdown(wit_bindgen_gen_markdown::Opts),
    /// Generates bindings for Rust guest modules.
    #[cfg(feature = "rust")]
    Rust(wit_bindgen_gen_guest_rust::Opts),
    /// Generates bindings for C/CPP guest modules.
    #[cfg(feature = "c")]
    C(wit_bindgen_gen_guest_c::Opts),
    /// Generates bindings for TeaVM-based Java guest modules.
    #[cfg(feature = "teavm-java")]
    TeavmJava(wit_bindgen_gen_guest_teavm_java::Opts),
}

#[derive(Debug, Parser)]
struct WorldOpt {
    /// WIT document to generate bindings for.
    #[clap(value_name = "DOCUMENT")]
    wit: PathBuf,

    /// World within the WIT document specified to generate bindings for.
    ///
    /// This can either be `foo` which is the default world in document `foo` or
    /// it's `foo.bar` which is the world named `bar` within document `foo`.
    #[clap(short, long)]
    world: Option<String>,
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

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    let mut files = Files::default();
    let generator = match opt.category {
        #[cfg(feature = "rust")]
        Category::Rust(opts) => opts.build(),
        #[cfg(feature = "c")]
        Category::C(opts) => opts.build(),
        #[cfg(feature = "teavm-java")]
        Category::TeavmJava(opts) => opts.build(),
        #[cfg(feature = "markdown")]
        Category::Markdown(opts) => opts.build(),
    };

    gen_world(generator, opt.world, &mut files)?;

    for (name, contents) in files.iter() {
        let dst = match &opt.common.out_dir {
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
    let mut resolve = Resolve::default();
    let pkg = if opts.wit.is_dir() {
        resolve.push_dir(&opts.wit)?.0
    } else {
        resolve.push(
            UnresolvedPackage::parse_file(&opts.wit)?,
            &Default::default(),
        )?
    };
    let world = match &opts.world {
        Some(world) => {
            let mut parts = world.splitn(2, '.');
            let doc = parts.next().unwrap();
            let world = parts.next();
            let doc = *resolve.packages[pkg]
                .documents
                .get(doc)
                .ok_or_else(|| anyhow!("no document named `{doc}` in package"))?;
            match world {
                Some(name) => *resolve.documents[doc]
                    .worlds
                    .get(name)
                    .ok_or_else(|| anyhow!("no world named `{name}` in document"))?,
                None => resolve.documents[doc]
                    .default_world
                    .ok_or_else(|| anyhow!("no default world in document"))?,
            }
        }
        None => {
            let mut docs = resolve.packages[pkg].documents.iter();
            let (_, doc) = docs
                .next()
                .ok_or_else(|| anyhow!("no documents found in package"))?;
            if docs.next().is_some() {
                bail!("multiple documents found in package, specify which to bind with `--world` argument")
            }
            resolve.documents[*doc]
                .default_world
                .ok_or_else(|| anyhow!("no default world in document"))?
        }
    };
    generator.generate(&resolve, world, files);
    Ok(())
}
