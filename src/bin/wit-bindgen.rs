use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use wit_bindgen_gen_core::{wit_parser, Files, Generator};
use wit_parser::Interface;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    RustWasm {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_rust_wasm::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Wasmtime {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_wasmtime::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    WasmtimePy {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_wasmtime_py::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Js {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_js::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    C {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_c::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Markdown {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_markdown::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    #[structopt(name = "spidermonkey")]
    SpiderMonkey {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_spidermonkey::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Wasmer {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_wasmer::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    WasmerPy {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_wasmer_py::Opts,
        #[structopt(flatten)]
        common: Common,
    },
}

#[derive(Debug, StructOpt)]
struct Common {
    /// Where to place output files
    #[structopt(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Generate import bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[structopt(long = "import", short)]
    imports: Vec<PathBuf>,

    /// Generate export bindings for the given `*.wit` interface. Can be
    /// specified multiple times.
    #[structopt(long = "export", short)]
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
        Command::SpiderMonkey { opts, common } => {
            let js_source = std::fs::read_to_string(&opts.js)
                .with_context(|| format!("failed to read {}", opts.js.display()))?;
            (Box::new(opts.build(js_source)), common)
        }
        Command::Wasmer { opts, common } => (Box::new(opts.build()), common),
        Command::WasmerPy { opts, common } => (Box::new(opts.build()), common),
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
