use anyhow::{bail, Context, Error, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str;
use wit_bindgen_core::{wit_parser, Files, WorldGenerator};
use wit_parser::Resolve;

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
        opts: wit_bindgen_markdown::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for MoonBit guest modules.
    #[cfg(feature = "moonbit")]
    Moonbit {
        #[clap(flatten)]
        opts: wit_bindgen_moonbit::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for Rust guest modules.
    #[cfg(feature = "rust")]
    Rust {
        #[clap(flatten)]
        opts: wit_bindgen_rust::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for C/CPP guest modules.
    #[cfg(feature = "c")]
    C {
        #[clap(flatten)]
        opts: wit_bindgen_c::Opts,
        #[clap(flatten)]
        args: Common,
    },
    /// Generates bindings for C++ modules.
    #[cfg(feature = "cpp")]
    Cpp {
        #[clap(flatten)]
        opts: wit_bindgen_cpp::Opts,
        #[clap(flatten)]
        args: Common,
    },

    /// Generates bindings for TinyGo-based Go guest modules (Deprecated)
    #[cfg(feature = "go")]
    TinyGo {
        #[clap(flatten)]
        args: Common,
    },

    /// Generates bindings for C# guest modules.
    #[cfg(feature = "csharp")]
    #[command(alias = "c-sharp")]
    Csharp {
        #[clap(flatten)]
        opts: wit_bindgen_csharp::Opts,
        #[clap(flatten)]
        args: Common,
    },

    // doc-comments are present on `wit_bindgen_test::Opts` for clap to use.
    Test {
        #[clap(flatten)]
        opts: wit_bindgen_test::Opts,
    },
}

#[derive(Debug, Parser)]
struct Common {
    /// Where to place output files
    #[clap(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Locations of WIT file(s) to generate bindings for.
    ///
    /// These paths can be either directories containing `*.wit` files, `*.wit`
    /// files themselves, or `*.wasm` files which are wasm-encoded WIT packages.
    /// Most of the time they're likely to be directories containing `*.wit`
    /// files with optional `deps` folders inside of them.
    ///
    /// Currently these locations must be ordered, as later paths can't contain
    /// dependencies on earlier paths. This restriction may be lifted in the
    /// future.
    #[clap(value_name = "WIT", index = 1)]
    wit: Vec<PathBuf>,

    /// Optionally specified world that bindings are generated for.
    ///
    /// Bindings are always generated for a world but this option can be omitted
    /// when the WIT package pointed to by the `WIT` option only has a single
    /// world. If there's more than one world in the package then this option
    /// must be specified to name the world that bindings are generated for.
    /// This option can also use the fully qualified syntax such as
    /// `wasi:http/proxy` to select a world from a dependency of the main WIT
    /// package.
    #[clap(short, long)]
    world: Option<String>,

    /// Indicates that no files are written and instead files are checked if
    /// they're up-to-date with the source files.
    #[clap(long)]
    check: bool,

    /// Comma-separated list of features that should be enabled when processing
    /// WIT files.
    ///
    /// This enables using `@unstable` annotations in WIT files.
    #[clap(long)]
    features: Vec<String>,

    /// Whether or not to activate all WIT features when processing WIT files.
    ///
    /// This enables using `@unstable` annotations in WIT files.
    #[clap(long)]
    all_features: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let mut files = Files::default();
    let (generator, opt) = match Opt::parse() {
        #[cfg(feature = "markdown")]
        Opt::Markdown { opts, args } => (opts.build(), args),
        #[cfg(feature = "moonbit")]
        Opt::Moonbit { opts, args } => (opts.build(), args),
        #[cfg(feature = "c")]
        Opt::C { opts, args } => (opts.build(), args),
        #[cfg(feature = "cpp")]
        Opt::Cpp { opts, args } => (opts.build(args.out_dir.as_ref()), args),
        #[cfg(feature = "rust")]
        Opt::Rust { opts, args } => (opts.build(), args),
        #[cfg(feature = "go")]
        Opt::TinyGo { args: _ } => {
            bail!("Go bindgen has been moved to a separate repository. Please visit https://github.com/bytecodealliance/go-modules for the new Go bindings generator `wit-bindgen-go`.")
        }
        #[cfg(feature = "csharp")]
        Opt::Csharp { opts, args } => (opts.build(), args),
        Opt::Test { opts } => return opts.run(std::env::args_os().nth(0).unwrap().as_ref()),
    };

    gen_world(generator, &opt, &mut files).map_err(attach_with_context)?;

    for (name, contents) in files.iter() {
        let dst = match &opt.out_dir {
            Some(path) => path.join(name),
            None => name.into(),
        };
        eprintln!("Generating {:?}", dst);

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

fn attach_with_context(err: Error) -> Error {
    #[cfg(feature = "rust")]
    if let Some(e) = err.downcast_ref::<wit_bindgen_rust::MissingWith>() {
        let option = e.0.clone();
        return err.context(format!(
            "missing either `--generate-all` or `--with {option}=(...|generate)`"
        ));
    }
    err
}

fn gen_world(
    mut generator: Box<dyn WorldGenerator>,
    opts: &Common,
    files: &mut Files,
) -> Result<()> {
    let mut resolve = Resolve::default();
    resolve.all_features = opts.all_features;
    for features in opts.features.iter() {
        for feature in features
            .split(',')
            .flat_map(|s| s.split_whitespace())
            .filter(|f| !f.is_empty())
        {
            resolve.features.insert(feature.to_string());
        }
    }
    let mut main_packages = Vec::new();
    for wit in &opts.wit {
        let (pkg, _files) = resolve.push_path(wit)?;
        main_packages.push(pkg);
    }
    let world = resolve.select_world(&main_packages, opts.world.as_deref())?;
    generator.generate(&resolve, world, files)?;

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
