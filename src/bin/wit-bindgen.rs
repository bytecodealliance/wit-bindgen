use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str;
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

    /// Indicates that no files are written and instead files are checked if
    /// they're up-to-date with the source files.
    #[clap(long)]
    check: bool,
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
    };

    gen_world(generator, &opt, &mut files)?;

    for (name, contents) in files.iter() {
        let dst = match &opt.out_dir {
            Some(path) => path.join(name),
            None => name.into(),
        };
        println!("Generating {:?}", dst);

        if opt.check {
            let prev = std::fs::read(&dst).with_context(|| format!("failed to read {:?}", dst))?;
            if prev != contents {
                // The contents differ. If it looks like textual contents, do a
                // line-by-line comparison so that we can tell users what the
                // problem is directly.
                if let (Ok(utf8_prev), Ok(utf8_contents)) =
                    (str::from_utf8(&prev), str::from_utf8(contents))
                {
                    if !utf8_prev
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                        && utf8_prev.lines().eq(utf8_contents.lines())
                    {
                        bail!("{} differs only in line endings (CRLF vs. LF). If this is a text file, configure git to mark the file as `text eol=lf`.", dst.display());
                    }
                }
                // The contents are binary or there are other differences; just
                // issue a generic error.
                bail!("not up to date: {}", dst.display());
            }
            continue;
        }

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
    opts: &Common,
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
            if resolve.packages[pkg].documents.is_empty() {
                bail!("no documents found in package")
            }

            let mut unique_default_world = None;
            for (_name, doc) in &resolve.documents {
                if let Some(default_world) = doc.default_world {
                    if unique_default_world.is_some() {
                        bail!("multiple default worlds found in package, specify which to bind with `--world` argument")
                    } else {
                        unique_default_world = Some(default_world);
                    }
                }
            }

            unique_default_world.ok_or_else(|| anyhow!("no default world in package"))?
        }
    };
    generator.generate(&resolve, world, files);
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
