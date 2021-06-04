#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
    };
    use wasmlink::{Linker, Module, Profile};
    use wasmtime_wasi::{Wasi, WasiCtxBuilder};

    fn module_path(name: &str) -> PathBuf {
        Path::new("modules/target/wasm32-wasi")
            .join(if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            })
            .join(format!("{}.wasm", name))
    }

    fn witx_path(name: &str) -> PathBuf {
        Path::new("modules/crates")
            .join(name)
            .join(format!("{}.witx", name))
    }

    pub fn link(main: &str, imports: &[&str]) -> Result<Vec<u8>> {
        let main_bytes = fs::read(module_path(main))?;

        let main_module = Module::new("main", &main_bytes)?;

        let imports: HashMap<&str, Vec<u8>> = imports
            .iter()
            .map(|name| {
                fs::read(module_path(name))
                    .map(|bytes| (*name, bytes))
                    .map_err(Into::into)
            })
            .collect::<Result<HashMap<&str, Vec<u8>>>>()?;

        let import_modules: HashMap<&str, Module> = imports
            .iter()
            .map(|(name, bytes)| {
                Module::new(name, bytes).and_then(|mut m| {
                    let path = witx_path(name);
                    if path.is_file() {
                        m.read_interface(&path)?;
                    }
                    Ok((name.as_ref(), m))
                })
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let linker = Linker::new(Profile::new());
        linker.link(&main_module, &import_modules)
    }

    pub fn run(module: &[u8]) -> Result<()> {
        use wasmtime::{Config, Engine, Module, Store};

        let mut config = Config::new();
        config.wasm_module_linking(true);
        config.wasm_multi_memory(true);

        Wasi::add_to_config(&mut config);

        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, module)?;
        let store = Store::new(&engine);

        assert!(Wasi::set_context(&store, WasiCtxBuilder::new().build()).is_ok());

        let linker = wasmtime::Linker::new(&store);
        let instance = linker.instantiate(&module)?;
        let start = instance.get_typed_func::<(), ()>("_start")?;

        start.call(())?;

        Ok(())
    }

    #[test]
    fn basic_types() -> Result<()> {
        run(&link("types-main", &["types"])?)?;

        Ok(())
    }
}
