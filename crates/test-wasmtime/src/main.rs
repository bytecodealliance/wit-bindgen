use wasmtime::*;

const WASM: &[u8] = include_bytes!(env!("WASM"));

mod host {
    witx_bindgen_wasmtime::import!("tests/host.witx");
}

struct MyHost;

impl host::Host for MyHost {
    fn roundtrip_u8(&self, val: u8) -> u8 {
        val
    }

    fn roundtrip_s8(&self, val: i8) -> i8 {
        val
    }
}

fn main() -> anyhow::Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, WASM)?;
    let store = Store::new(&engine);
    let mut linker = Linker::new(&store);
    host::add_host_to_linker(MyHost, &mut linker)?;
    wasmtime_wasi::Wasi::new(
        &store,
        wasi_cap_std_sync::WasiCtxBuilder::new()
            .inherit_stdio()
            .build()?,
    )
    .add_to_linker(&mut linker)?;
    let instance = linker.instantiate(&module)?;
    Ok(())
}
