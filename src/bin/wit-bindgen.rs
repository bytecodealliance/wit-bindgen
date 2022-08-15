use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use wit_bindgen_core::{wit_parser, Files, Generator};
use wit_parser::Interface;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    category: Category,
}

#[derive(Debug, StructOpt)]
enum Category {
    Host(HostGenerator),
    Guest(GuestGenerator),
    Markdown {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_markdown::Opts,
        #[structopt(flatten)]
        common: Common,
    }
}

#[derive(Debug, StructOpt)]
enum HostGenerator {
    Wasmtime {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_rust::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    WasmtimePy {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_host_wasmtime_py::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    Js {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_host_js::Opts,
        #[structopt(flatten)]
        common: Common,
    },
}

#[derive(Debug, StructOpt)]
enum GuestGenerator {
    Rust {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_guest_rust::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    C {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_guest_c::Opts,
        #[structopt(flatten)]
        common: Common,
    },
    #[structopt(name = "spidermonkey")]
    SpiderMonkeyJS {
        #[structopt(flatten)]
        opts: wit_bindgen_gen_guest_spidermonkey_js::Opts,
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
    let opt: Opt = Opt::from_args();
    let (mut generator, common): (Box<dyn Generator>, _) = match opt.category {
        Category::Guest(GuestGenerator::Rust { opts, common }) => (Box::new(opts.build()), common),
        Category::Host(HostGenerator::Wasmtime { opts, common }) => (Box::new(opts.build()), common),
        Category::Host(HostGenerator::WasmtimePy { opts, common }) => (Box::new(opts.build()), common),
        Category::Host(HostGenerator::Js { opts, common }) => (Box::new(opts.build()), common),
        Category::Guest(GuestGenerator::C { opts, common }) => (Box::new(opts.build()), common),
        Category::Markdown { opts, common } => (Box::new(opts.build()), common),
        Category::Guest(GuestGenerator::SpiderMonkeyJS { opts, common }) => {
            let js_source = std::fs::read_to_string(&opts.js)
                .with_context(|| format!("failed to read {}", opts.js.display()))?;
            (Box::new(opts.build(js_source)), common)
        }
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
