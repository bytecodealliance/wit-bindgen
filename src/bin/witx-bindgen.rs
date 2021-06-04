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
}

#[derive(Debug, StructOpt)]
struct Common {
    /// Where to place output files
    #[structopt(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Whether bindings are generated for as-if these functions are imported
    #[structopt(long, short, conflicts_with("export"))]
    import: bool,

    /// Whether bindings are generated for as-if these functions are exported
    #[structopt(long, short, conflicts_with("import"))]
    export: bool,

    /// Input `*.witx` files to create bindings for
    witx: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let (mut generator, common): (Box<dyn Generator>, _) = match opt.command {
        Command::RustWasm { opts, common } => (Box::new(opts.build()), common),
        Command::Wasmtime { opts, common } => (Box::new(opts.build()), common),
    };

    if !common.import && !common.export {
        anyhow::bail!("one of `--import` or `--export` must be used");
    }

    let mut files = Files::default();
    for witx in common.witx {
        let module = witx2::Interface::parse_file(witx)?;
        generator.generate(&module, common.import, &mut files);
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
