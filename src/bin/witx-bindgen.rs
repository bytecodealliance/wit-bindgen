use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use witx_bindgen_gen_core::{witx2, Files, Generator};

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    RustWasm {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_rust_wasm::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Wasmtime {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_wasmtime::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    WasmtimePy {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_wasmtime_py::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Js {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_js::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    C {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_c::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Markdown {
        #[structopt(flatten)]
        opts: witx_bindgen_gen_markdown::Opts,
        #[structopt(flatten)]
        common: Common,
    },
}

#[derive(Debug, StructOpt)]
struct Common {
    /// Where to place output files
    #[structopt(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Generate import binding for the given `*.witx` file. Can be specified
    /// multiple times.
    #[structopt(long, short)]
    imports: Vec<PathBuf>,

    /// Generate export binding for the given `*.witx` file. Can be specified
    /// multiple times.
    #[structopt(long, short)]
    exports: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let (mut generator, common): (Box<dyn Generator>, _) = match opt.command {
        Command::RustWasm { opts, common } => (Box::new(opts.build()), common),
        Command::Wasmtime { opts, common } => (Box::new(opts.build()), common),
        Command::WasmtimePy { opts, common } => (Box::new(opts.build()), common),
        Command::Js { opts, common } => (Box::new(opts.build()), common),
        Command::C { opts, common } => (Box::new(opts.build()), common),
        Command::Markdown { opts, common } => (Box::new(opts.build()), common),
    };

    let imports = common
        .imports
        .iter()
        .map(|witx| witx2::Interface::parse_file(witx))
        .collect::<Result<Vec<_>>>()?;
    let exports = common
        .exports
        .iter()
        .map(|witx| witx2::Interface::parse_file(witx))
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
