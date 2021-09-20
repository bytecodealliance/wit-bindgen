//! Testing and testing utilities for `witx-bindgen-spidermonkey`.

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use std::fs;
use std::path::{Path, PathBuf};
use witx_bindgen_gen_core::{witx2, Files, Generator};
use witx_bindgen_gen_spidermonkey::SpiderMonkeyWasm;

/// Get an iterator of all of our test
/// `witx-bindgen/crates/spidermonkey-test/tests/*.witx` files.
pub fn witx_files() -> Result<impl Iterator<Item = Result<PathBuf>>> {
    let dir = fs::read_dir("./tests").context("failed to read tests directory")?;
    Ok(dir.filter_map(|e| match e {
        Ok(e) => {
            let path = e.path();
            if path.is_file() && path.extension().and_then(|x| x.to_str()) == Some("witx") {
                Some(Ok(path))
            } else {
                None
            }
        }
        Err(e) => Some(Err(e.into())),
    }))
}

/// Generate Wasm glue for the given `.witx` file.
///
/// When treating the file as an import, use `foo.import.js` as the JS source
/// code (assuming the `.witx` file is named `foo.witx`). When treating the
/// `.witx` file as an export, use `foo.export.js` as the JS source code.
///
/// For debugging purposes, the generated Wasm will be written to
/// `foo.{imported,exported}.wasm` if debug logging is enabled.
pub fn generate(witx: &Path, dir: witx2::abi::Direction) -> Result<Vec<u8>> {
    let mut smw = SpiderMonkeyWasm::default();
    smw.import_spidermonkey(true);

    let modules = vec![witx2::Interface::parse_file(&witx)?];
    match dir {
        witx2::abi::Direction::Import => smw.preprocess_all(&modules, &[]),
        witx2::abi::Direction::Export => smw.preprocess_all(&[], &modules),
    }

    let mut files = Files::default();
    smw.generate(&modules[0], dir, &mut files);

    let mut js = witx.to_owned();
    js.set_extension(match dir {
        witx2::abi::Direction::Import => "import.js",
        witx2::abi::Direction::Export => "export.js",
    });
    let js_source =
        fs::read_to_string(&js).with_context(|| format!("failed to read {}", js.display()))?;
    let wasm = smw.into_wasm(&js.display().to_string(), &js_source);

    if log::log_enabled!(log::Level::Debug) {
        let mut wasm_file = js;
        wasm_file.set_extension("wasm");
        log::debug!("writing generated Wasm to {}", wasm_file.display());
        std::fs::write(&wasm_file, &wasm)
            .with_context(|| format!("failed to write {}", wasm_file.display()))?;
    }

    Ok(wasm)
}

/// Compile, instantiate, and initialize the import and export versions of the
/// given WITX file.
///
/// For the WITX file `path/to/interface.witx`, the JS exporting this interface
/// must live at `path/to/interface.export.js` and the JS importing this
/// interface must live at `path/to/interface.import.js`.
///
/// The JS importing the WITX should call imports and run assertions and all
/// that in its top-level code so that it runs during the `wizer.initialize`
/// call. Then the test code can assert that the expected things were called and
/// that the JS didn't throw any exceptions.
///
/// Callers of this function can also test the export JS version by poking at
/// the exported interface and ensuring that it behaves correctly.
pub fn run_test<T>(
    witx: impl AsRef<Path>,
    cx: T,
    mut make_linker: impl FnMut(&wasmtime::Engine) -> Result<wasmtime::Linker<T>>,
) -> Result<(
    wasmtime::Store<T>,
    wasmtime::Linker<T>,
    wasmtime::Module,
    wasmtime::Instance,
)> {
    drop(env_logger::try_init());

    let witx = witx.as_ref();

    // Make sure we only compile `spidermonkey.wasm` once per process, not once
    // per test.
    lazy_static! {
        static ref CONFIG: wasmtime::Config = {
            let mut config = wasmtime::Config::new();
            config
                .wasm_module_linking(true)
                .wasm_multi_memory(true)
                .wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable)
                .cache_config_load_default()
                .context("failed to load cache config")
                .unwrap();
            config
        };
        static ref ENGINE: wasmtime::Engine = {
            wasmtime::Engine::new(&CONFIG)
                .context("failed to create ENGINE")
                .unwrap()
        };
        static ref SMW_MODULE: wasmtime::Module = {
            log::debug!("reading `spidermonkey.wasm`");
            let smw_bytes = std::fs::read("../../spidermonkey-wasm/spidermonkey.wasm")
                .context("failed to read `spidermonkey.wasm`")
                .unwrap();

            log::debug!("compiling `spidermonkey.wasm`");
            let smw_module =
                wasmtime::Module::new_with_name(&ENGINE, &smw_bytes, "spidermonkey.wasm")
                    .context("failed to compile `spidermonkey.wasm`")
                    .unwrap();

            smw_module
        };
    }

    let import_wasm = generate(witx, witx2::abi::Direction::Import)
        .context("failed to generate import bindings")?;

    log::debug!("compiling `import.wasm`");
    let import_module = wasmtime::Module::new_with_name(&ENGINE, &import_wasm, "import.wasm")
        .context("failed to compile import module")?;

    let export_wasm = generate(witx, witx2::abi::Direction::Export)
        .context("failed to generate export bindings")?;

    log::debug!("compiling `export.wasm`");
    let export_module = wasmtime::Module::new_with_name(&ENGINE, &export_wasm, "export.wasm")
        .context("failed to compile export module")?;

    // Instantiate `spidermonkey.wasm` (triggering it to be read and compiled,
    // if we are the first test to get this far) only after we've already
    // compiled the generated import and export wasm. This allows us to more
    // quickly catch bugs in the generator without having to wait on
    // `spidermonkey.wasm`.

    let mut store = wasmtime::Store::new(&ENGINE, cx);

    let mut make_linker_with_smw = |store: &mut wasmtime::Store<T>| -> Result<wasmtime::Linker<T>> {
        let mut linker = make_linker(&ENGINE)?;

        // Instantiate `spidermonkey.wasm` and put it in the linker.
        log::debug!("instantiating `spidermonkey.wasm");
        let smw_instance = linker
            .instantiate(&mut *store, &SMW_MODULE)
            .context("failed to instantiate `spidermonkey.wasm`")?;

        linker.define_name("spidermonkey", smw_instance)?;

        Ok(linker)
    };

    // When the Wasm is importing the WITX interface, we instantiate the
    // generated module and run its `wizer.initialize` function to evaluate the
    // top-level of the JS. The top-level JS will call imported functions and
    // make assertions; after its evaluation, we are done with this instance.
    {
        let import_linker = make_linker_with_smw(&mut store)?;

        log::debug!("instantiating `import.wasm`");
        let import_instance = import_linker
            .instantiate(&mut store, &import_module)
            .context("failed to instantiate import module")?;

        log::debug!("calling `wizer.initialize` on `import.wasm` instance");
        import_instance
            .get_typed_func::<(), (), _>(&mut store, "wizer.initialize")?
            .call(&mut store, ())
            .context("failed to initialize import module via `wizer.initialize`")?;
    }

    let export_linker = make_linker_with_smw(&mut store)?;

    log::debug!("instantiating `export.wasm`");
    let export_instance = export_linker
        .instantiate(&mut store, &export_module)
        .context("fialed to instantiate export module")?;

    log::debug!("calling `wizer.initialize` on `export.wasm` instance");
    export_instance
        .get_typed_func::<(), (), _>(&mut store, "wizer.initialize")?
        .call(&mut store, ())
        .context("failed to initialize import module via `wizer.initialize`")?;

    Ok((store, export_linker, export_module, export_instance))
}
