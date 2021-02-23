use anyhow::{Error, Result};
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    /// One of `export` or `import`
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    Import {
        #[structopt(flatten)]
        files: Files,
    },
    Export {
        #[structopt(flatten)]
        files: Files,
    },
}

#[derive(Debug, StructOpt)]
struct Files {
    /// Where to place output files
    #[structopt(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Input `*.witx` files to create bindings for
    witx: Vec<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
}
