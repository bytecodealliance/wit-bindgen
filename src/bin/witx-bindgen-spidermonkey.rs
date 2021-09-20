use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;
use witx_bindgen_gen_core::{witx2, Files, Generator};
use witx_bindgen_gen_spidermonkey::SpiderMonkeyWasm;

#[derive(Debug, StructOpt)]
struct Options {
    /// Import a `spidermonkey.wasm` instance, rather than embedding a
    /// `spidermonkey.wasm` module.
    #[structopt(long)]
    import_spidermonkey: bool,

    /// Where to place output files
    #[structopt(long = "out-dir")]
    out_dir: Option<PathBuf>,

    /// Generate import binding for the given `*.witx` file. Can be specified
    /// multiple times.
    #[structopt(long = "import", short)]
    imports: Vec<PathBuf>,

    /// Generate export binding for the given `*.witx` file. Can be specified
    /// multiple times.
    #[structopt(long = "export", short)]
    exports: Vec<PathBuf>,

    /// The JavaScript file.
    js: PathBuf,
}

fn main() -> Result<()> {
    let options = Options::from_args();

    let mut generator = SpiderMonkeyWasm::default();
    generator.import_spidermonkey(options.import_spidermonkey);

    let mut files = Files::default();

    let mut imports = vec![];
    for witx in &options.imports {
        imports.push(witx2::Interface::parse_file(witx)?);
    }
    let mut exports = vec![];
    for witx in &options.exports {
        exports.push(witx2::Interface::parse_file(witx)?);
    }
    anyhow::ensure!(
        exports.len() <= 1,
        "Only at most one export interface is currently supported"
    );

    generator.preprocess_all(&imports, &exports);

    for module in imports {
        generator.generate(&module, witx2::abi::Direction::Import, &mut files);
    }

    for module in exports {
        generator.generate(&module, witx2::abi::Direction::Export, &mut files);
    }

    let js = std::fs::read_to_string(&options.js)
        .with_context(|| format!("failed to read {}", options.js.display()))?;
    let wasm = generator.into_wasm(&options.js.display().to_string(), &js);

    let js_file_stem = options.js.file_stem().ok_or_else(|| {
        anyhow::anyhow!(
            "input JavaScript file path does not have a file stem: {}",
            options.js.display()
        )
    })?;
    let js_file_stem = js_file_stem.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "input JavaScript file path is not UTF-8 representable: {}",
            options.js.display()
        )
    })?;
    let wasm_name = format!("{}.wasm", js_file_stem);

    files.push(&wasm_name, &wasm);

    for (name, contents) in files.iter() {
        let dst = match &options.out_dir {
            Some(path) => path.join(name),
            None => name.into(),
        };
        println!("Writing {:?}", dst);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {:?}", parent))?;
        }
        std::fs::write(&dst, contents).with_context(|| format!("failed to write {:?}", dst))?;
    }

    Ok(())
}
